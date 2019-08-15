extern crate chrono;
extern crate failure;
extern crate libc;
extern crate structopt;
extern crate x11;

use chrono::Local;
use failure::Error;
use libc::{c_char, c_int, c_long, c_uint, c_void};
use std::convert::TryInto;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::mem;
use std::path::PathBuf;
use std::{ptr, str};
use structopt::StructOpt;
use x11::keysym;
use x11::xlib;
use x11::xlib::*;

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Grabs keyboard and behaves like an append-only text editor until you press ctrl+alt+escape."
)]
struct Opt {
    #[structopt(long = "debug")]
    debug: bool,
    #[structopt(parse(from_os_str), help = "File to write text to")]
    output_file: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    // TODO: When appending to file, parse a prefix of it as utf8 as a sanity check.
    let output_file_name = match opt.output_file.clone() {
        Some(output_file_name) => output_file_name,
        None => PathBuf::from(Local::now().format("night-%Y-%m-%d").to_string()),
    };
    let mut output_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_file_name)
        .unwrap();
    let mut output_string = String::new();
    let append_start = output_file.metadata().unwrap().len();
    let display = open_display();
    let root = default_root_window(display);
    let xim = open_im(display);
    let xic = create_ic(xim, XIMPreeditNothing | XIMStatusNothing, root.into());
    set_ic_focus(xic);
    grab_keyboard(display, root);
    let mut x_event = unsafe { mem::zeroed() };
    loop {
        unsafe {
            XNextEvent(display, &mut x_event);
        }
        let result = match unsafe { x_event.type_ } {
            xlib::KeyPress => {
                let event: &XKeyEvent = unsafe { mem::transmute(&x_event) };
                if filter_event(&x_event, root) {
                    continue;
                }
                handle_keypress(
                    &opt,
                    xic,
                    event,
                    append_start,
                    &mut output_file,
                    &mut output_string,
                )
            }
            _ => Ok(HandlerResult::KeepGoing),
        };
        match result {
            Ok(HandlerResult::KeepGoing) => {}
            Ok(HandlerResult::Exit) => break,
            Err(e) => {
                eprintln!("nightwriter: ignoring error: {:?}", e);
            }
        }
    }
    // TODO: cleanup gracefully on exceptions?
    // Note this cleanup probably isn't even necessary.
    ungrab_keyboard(display);
    destroy_ic(xic);
    close_im(xim);
    close_display(display);
}

enum HandlerResult {
    KeepGoing,
    Exit,
}

// NOTE: I acknowledge that this function probably ought to be split up :)
fn handle_keypress(
    opt: &Opt,
    xic: XIC,
    event: &XKeyEvent,
    append_start: u64,
    output_file: &mut File,
    output_string: &mut String,
) -> Result<HandlerResult, Error> {
    let ctrl = event.state & ControlMask != 0;
    let shift = event.state & ShiftMask != 0;
    let keysym = lookup_keysym(&mut event.clone(), 0);
    if ctrl && shift && keysym == Some(keysym::XK_Escape) {
        // NOTE: this is intentionally done early and before anything that can fail, with the hope
        // that nightwriter can never get in an unexitable state.
        return Ok(HandlerResult::Exit);
    }
    match keysym {
        Some(keysym::XK_BackSpace) => {
            let old_len = output_string.len();
            let new_len = if event.state & ControlMask != 0 {
                if opt.debug {
                    eprintln!("nightwriter: ctrl+backspace");
                }
                match output_string.chars().rev().nth(0) {
                    None => 0,
                    Some(last_char) => {
                        // NOTE: Standard ctrl+backspace pays attention to symbols and such, seems
                        // convoluted.  Instead just have it delete up till the last whitespace.
                        if last_char.is_whitespace() {
                            output_string.truncate(
                                output_string
                                    .rfind(|c: char| !c.is_whitespace())
                                    .map_or(0, |ix| ix + 1),
                            );
                        }
                        output_string
                            .rfind(|c: char| c.is_whitespace())
                            .map_or(0, |ix| ix + 1)
                    }
                }
            } else {
                if opt.debug {
                    eprintln!("nightwriter: backspace");
                }
                let len = output_string.len();
                if len > 0 {
                    len - 1
                } else {
                    len
                }
            };
            if opt.debug {
                eprintln!(
                    "nightwriter: deleted {} bytes, now at {}",
                    old_len - new_len,
                    new_len
                );
            }
            output_string.truncate(new_len);
            let new_file_len = append_start + new_len as u64;
            output_file.set_len(new_file_len)?;
            output_file.seek(SeekFrom::Start(new_file_len))?;
            return Ok(HandlerResult::KeepGoing);
        }
        _ => {}
    }
    let keystring = utf8_lookup_string(xic, &event);
    // Recognize various sequences that look like attempts to exit (ctrl+c, ctrl+d, ctrl+\, ctrl+z,
    // or escape).
    if keystring == "\u{3}"
        || keystring == "\u{4}"
        || keystring == "\u{1c}"
        || keystring == "\u{1a}"
        || keysym == Some(keysym::XK_Escape)
    {
        print_exit_instructions();
    }
    for in_chr in keystring.chars() {
        let optional_out_chr = if in_chr == '\r' {
            Some('\n')
        } else if in_chr == '\t' {
            Some('\t')
        } else if in_chr.is_control() {
            None
        } else {
            Some(in_chr)
        };
        match optional_out_chr {
            Some(out_chr) => {
                output_file.write_all(out_chr.to_string().as_bytes())?;
                output_string.push(out_chr);
                if opt.debug {
                    eprintln!(
                        "nightwriter: inserted {:?} at {}",
                        out_chr,
                        append_start + output_string.len() as u64
                    );
                }
            }
            None => {
                if opt.debug {
                    eprintln!("nightwriter: ignoring {:?}", in_chr);
                }
            }
        }
    }
    if opt.debug && keystring.len() == 0 {
        eprintln!(
            "nightwriter: ignoring keycode {} with keysym {:?}",
            event.keycode, keysym
        );
    }
    return Ok(HandlerResult::KeepGoing);
}

fn print_exit_instructions() {
    eprintln!("Press ctrl+shift+escape to exit nightwriter.");
}

fn grab_keyboard(display: *mut Display, root: Window) {
    let grab_result = unsafe {
        XGrabKeyboard(
            display,
            root,
            False,
            GrabModeAsync,
            GrabModeAsync,
            CurrentTime,
        )
    };
    if grab_result != GrabSuccess {
        panic!("XGrabKeyboard() failed");
    }
}

fn ungrab_keyboard(display: *mut Display) {
    unsafe { XUngrabKeyboard(display, CurrentTime) };
}

fn lookup_keysym(event: &mut XKeyEvent, index: c_int) -> Option<c_uint> {
    let result = unsafe { XLookupKeysym(event, index) };
    if result == NoSymbol.try_into().ok()? {
        None
    } else {
        Some(result.try_into().ok()?)
    }
}

// Following code copy+modified from
// https://github.com/skligys/rusty-cardboard/blob/a3367b65b2e36b3735d64c9f153e2ec8c44569db/src/x11/mod.rs

fn open_display() -> *mut Display {
    let display = unsafe { XOpenDisplay(ptr::null()) };
    if display.is_null() {
        panic!("XOpenDisplay() failed");
    }
    display
}

fn close_display(display: *mut Display) {
    unsafe {
        XCloseDisplay(display);
    }
}

fn default_root_window(display: *mut Display) -> Window {
    unsafe { XDefaultRootWindow(display) }
}

fn filter_event(event: &XEvent, window: Window) -> bool {
    let rc = unsafe { XFilterEvent(mem::transmute(event as *const XEvent), window) };
    rc != 0
}

fn utf8_lookup_string(ic: XIC, event: &XKeyEvent) -> String {
    let mut buffer: [u8; 16] = unsafe { [mem::uninitialized(); 16] };
    let count = unsafe {
        Xutf8LookupString(
            ic,
            mem::transmute(event as *const XKeyEvent),
            mem::transmute(buffer.as_mut_ptr()),
            buffer.len() as c_int,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    str::from_utf8(&buffer[..count as usize])
        .unwrap_or("")
        .to_string()
}

fn open_im(display: *mut Display) -> XIM {
    let im = unsafe { XOpenIM(display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
    if im.is_null() {
        panic!("XOpenIM() failed");
    }
    im
}

fn close_im(input_method: XIM) {
    unsafe {
        XCloseIM(input_method);
    }
}

fn create_ic(im: XIM, input_style: i32, client_window: Window) -> XIC {
    let c_input_style_name = CString::new("inputStyle").unwrap();
    let c_client_window_name = CString::new("clientWindow").unwrap();
    let ic = unsafe {
        XCreateIC(
            im,
            c_input_style_name.as_ptr(),
            input_style.into(),
            c_client_window_name.as_ptr(),
            client_window,
            ptr::null(),
        )
    };
    if ic.is_null() {
        panic!("XCreateIC() failed");
    }
    ic
}

fn set_ic_focus(ic: XIC) {
    unsafe {
        XSetICFocus(ic);
    }
}

fn destroy_ic(ic: XIC) {
    unsafe {
        XDestroyIC(ic);
    }
}

#[link(name = "X11")]
extern "C" {
    fn XCreateIC(
        im: XIM,
        a: *const c_char,
        b: c_long,
        c: *const c_char,
        d: Window,
        e: *const c_void,
    ) -> XIC;
}
