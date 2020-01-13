use crate::mod_keys::ModKeys;
use failure::Error;
use libc::{c_char, c_int, c_long, c_uint, c_void};
use std::convert::TryInto;
use std::ffi::CString;
use std::mem;
use std::{ptr, str};
use x11::xlib;
use x11::xlib::*;

pub struct KeyPress {
    pub mod_keys: ModKeys,
    pub key_code: u32,
    pub key_sym: Option<u32>,
    pub key_string: String,
}

pub enum HandlerResult<T> {
    KeepGoing,
    Exit(T),
}

pub fn with_keyboard_grabbed<T>(
    handle_keypress: &Fn(KeyPress) -> Result<HandlerResult<T>, Error>,
) -> T {
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
        let handler_result = match unsafe { x_event.type_ } {
            xlib::KeyPress => {
                let event: &XKeyEvent = unsafe { mem::transmute(&x_event) };
                if filter_event(&x_event, root) {
                    continue;
                }
                handle_keypress(event_to_keypress(xic, &event))
            }
            _ => Ok(HandlerResult::KeepGoing),
        };
        match handler_result {
            Ok(HandlerResult::KeepGoing) => {}
            Ok(HandlerResult::Exit(result)) => {
                // Note this cleanup probably isn't even necessary.
                unsafe {
                    XUngrabKeyboard(display, CurrentTime);
                    XDestroyIC(xic);
                    XCloseIM(xim);
                    XCloseDisplay(display);
                }
                return result;
            }
            Err(e) => {
                eprintln!("nightwriter: ignoring error: {:?}", e);
            }
        }
    }
}

fn event_to_keypress(xic: XIC, event: &XKeyEvent) -> KeyPress {
    KeyPress {
        mod_keys: ModKeys {
            ctrl: event.state & ControlMask != 0,
            shift: event.state & ShiftMask != 0,
        },
        key_code: event.keycode,
        key_sym: lookup_keysym(&mut event.clone(), 0),
        key_string: utf8_lookup_string(xic, &event),
    }
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
