use failure::Error;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use toml;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub output_file_template: Option<String>,
    pub on_start: Option<BashCommand>,
    pub on_end: Option<BashCommand>,
    pub bindings: Bindings,
}

#[derive(Deserialize, Debug)]
pub struct Bindings(pub HashMap<KeyboardShortcut, BashCommand>);

#[derive(Deserialize, Debug)]
pub struct BashCommand(pub String);

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct KeyboardShortcut {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: char,
}

impl<'de> Deserialize<'de> for KeyboardShortcut {
    fn deserialize<D>(deserializer: D) -> Result<KeyboardShortcut, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let string = String::deserialize(deserializer)?;
        let chunks = string.split("-").collect::<Vec<_>>();
        for chunk in chunks[0..chunks.len() - 1].iter() {
            if *chunk == "C" {
                ctrl = true;
            } else if *chunk == "S" {
                shift = true;
            } else if *chunk == "M" || *chunk == "A" {
                alt = true;
            } else {
                // FIXME: proper error
                panic!(
                    "Unexpected modifier (expected C for Ctrl, S for Shift, or M for Alt): {}",
                    chunk
                );
            }
        }
        let last_chunk = chunks.last().unwrap_or(&"");
        if last_chunk.len() != 1 {
            // FIXME: proper error
            panic!(
                "Expected keyboard shortcut key to be just one ascii char, instad got: {}",
                last_chunk
            );
        }
        let key = last_chunk.chars().nth(0).unwrap();
        return Ok(KeyboardShortcut {
            ctrl,
            alt,
            shift,
            key,
        });
    }
}

fn default_config() -> Config {
    Config {
        output_file_template: None,
        on_start: None,
        on_end: None,
        bindings: Bindings(HashMap::new()),
    }
}

pub fn read_config(opt_config_path: Option<PathBuf>) -> Result<Config, Error> {
    let config_path = opt_config_path.unwrap_or(PathBuf::from(r"~/.nightwriter.toml"));
    match fs::read_to_string(config_path) {
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                Ok(default_config())
            } else {
                Err(e)?
            }
        }
        Ok(config_contents) => Ok(toml::from_str(&config_contents)?),
    }
}
