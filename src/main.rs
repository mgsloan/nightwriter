extern crate chrono;
extern crate failure;
extern crate libc;
extern crate structopt;
extern crate x11;

use chrono::Local;
use std::path::PathBuf;
use structopt::StructOpt;
use x11::keysym;

mod config;
mod grab_keyboard;
mod mod_keys;
mod writer;

use crate::writer::Writer;
use crate::mod_keys::ModKeys;
use crate::config::read_config;
use crate::grab_keyboard::{with_keyboard_grabbed, HandlerResult, KeyPress};

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Grabs keyboard and behaves like an append-only text editor until you press ctrl+alt+escape."
)]
pub struct Opt {
    #[structopt(long = "debug")]
    pub debug: bool,
    #[structopt(parse(from_os_str), help = "File to write text to")]
    pub output_file: Option<PathBuf>,
    #[structopt(parse(from_os_str), help = "Override configuration file location")]
    pub config: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    let config = match read_config(opt.config.clone()) {
        Err(e) => {
            println!("Failed to read nightwriter configuration:\n{}", e);
            return ();
        }
        Ok(config) => config,
    };
    let output_file_name = match opt.output_file.clone() {
        Some(output_file_name) => output_file_name,
        None => PathBuf::from(Local::now().format("night-%Y-%m-%d").to_string()),
    };
    let writer = Writer::initialize(output_file_name).unwrap();
    with_keyboard_grabbed(&|keypress| {
        let KeyPress {
            mod_keys: ModKeys { ctrl, shift },
            key_code,
            key_sym,
            key_string,
        } = keypress;
        if ctrl && shift && key_sym == Some(keysym::XK_Escape) {
            // NOTE: this is intentionally done early and before anything that can fail, with the
            // hope that nightwriter can never get in an unexitable state.
            return Ok(HandlerResult::Exit);
        }
        if key_sym == Some(keysym::XK_BackSpace) {
            if ctrl {
                if opt.debug {
                    eprintln!("nightwriter: delete word");
                }
                writer.delete_word()?;
            } else {
                if opt.debug {
                    eprintln!("nightwriter: delete char");
                }
                writer.delete_char()?;
            }
        } else if looks_like_exit(&key_string, &key_sym) {
            eprintln!("Press ctrl+shift+escape to exit nightwriter.");
        } else if key_string.len() > 0 {
            for chr in key_string.chars() {
                if should_insert(chr) {
                    if opt.debug {
                        eprintln!("nightwriter: append {:?}", chr);
                    }
                    writer.append(chr)?;
                } else if opt.debug {
                    eprintln!("nightwriter: ignoring {:?}", chr);
                }
            }
        } else if opt.debug {
            eprintln!(
                "nightwriter: ignoring keycode {} with keysym {:?}",
                key_code, key_sym
            );
        }
        Ok(HandlerResult::KeepGoing)
    });
}

fn should_insert(chr: char) -> bool {
    chr == '\r' || chr == '\t' || !chr.is_control()
}

// Recognize various sequences that look like attempts to exit (ctrl+c, ctrl+d, ctrl+\, ctrl+z, or
// escape).
fn looks_like_exit(key_string: &String, key_sym: &Option<u32>) -> bool {
    key_string == "\u{3}"
        || key_string == "\u{4}"
        || key_string == "\u{1c}"
        || key_string == "\u{1a}"
        || *key_sym == Some(keysym::XK_Escape)
}
