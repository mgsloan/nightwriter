use failure::Error;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::cell::RefCell;

pub struct Writer {
    append_start: u64,
    output_file: RefCell<File>,
    output_string: RefCell<String>,
}

impl Writer {
    pub fn initialize(output_file_name: &PathBuf) -> Result<Writer, Error> {
        let output_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_file_name)
            .unwrap();
        let output_string = String::new();
        let append_start = output_file.metadata().unwrap().len();
        Ok(Writer { append_start, output_file: RefCell::new(output_file), output_string: RefCell::new(output_string) })
    }

    pub fn append(&self, chr: char) -> Result<(), Error> {
        self.output_file.borrow_mut().write_all(chr.to_string().as_bytes())?;
        self.output_string.borrow_mut().push(chr);
        Ok(())
    }

    pub fn delete_char(&self) -> Result<(), Error> {
        let len = self.output_string.borrow().len();
        if len > 0 {
            self.truncate(len - 1)
        } else {
            self.truncate(len)
        }
    }

    pub fn delete_word(&self) -> Result<(), Error> {
        let new_len = match self.output_string.borrow().chars().rev().nth(0) {
            None => 0,
            Some(last_char) => {
                // NOTE: Standard ctrl+backspace pays attention to symbols and such, seems
                // convoluted.  Instead just have it delete up till the last whitespace.
                if last_char.is_whitespace() {
                    let truncate_len = self.output_string.borrow().rfind(|c: char| !c.is_whitespace())
                            .map_or(0, |ix| ix + 1);
                    self.output_string.borrow_mut().truncate(truncate_len);
                }
                self.output_string.borrow()
                    .rfind(|c: char| c.is_whitespace())
                    .map_or(0, |ix| ix + 1)
            }
        };
        self.truncate(new_len)
    }

    fn truncate(&self, new_len: usize) -> Result<(), Error> {
        self.output_string.borrow_mut().truncate(new_len);
        let new_file_len = self.append_start + new_len as u64;
        self.output_file.borrow().set_len(new_file_len)?;
        self.output_file.borrow_mut().seek(SeekFrom::Start(new_file_len))?;
        Ok(())
    }
}
