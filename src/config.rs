use failure::Error;
use crate::mod_keys::ModKeys;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use dirs;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub output_file_template: Option<String>,
    pub on_start: Option<BashCommand>,
    pub on_end: Option<BashCommand>,
    pub bindings: Option<Bindings>,
}

#[derive(Deserialize, Debug)]
pub struct Bindings(pub HashMap<KeyboardShortcut, BashCommand>);

#[derive(Deserialize, Debug)]
pub struct BashCommand(pub String);

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct KeyboardShortcut {
    pub mod_keys: ModKeys,
    pub key: char,
}

impl<'de> Deserialize<'de> for KeyboardShortcut {
    fn deserialize<D>(deserializer: D) -> Result<KeyboardShortcut, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut ctrl = false;
        let mut shift = false;
        let string = String::deserialize(deserializer)?;
        let chunks = string.split("-").collect::<Vec<_>>();
        for chunk in chunks[0..chunks.len() - 1].iter() {
            if *chunk == "C" {
                ctrl = true;
            } else if *chunk == "S" {
                shift = true;
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
            mod_keys: ModKeys {
                ctrl,
                shift,
            },
            key,
        });
    }
}

pub fn read_config(opt_config_path: Option<PathBuf>) -> Result<Config, Error> {
    let path = match opt_config_path {
        Some(path) => path,
        None => {
            match dirs::config_dir() {
                None => {
                    eprintln!("nightwriter: Neither XDG_CONFIG_HOME nor HOME environment variables are set, or --config option, so using default configuration");
                    return Ok(default_config())
                },
                Some(dir) => {
                    let mut path = dir.clone();
                    path.push("nightwriter");
                    path.push("config.toml");
                    path
                }
            }
        }
    };
    match fs::read_to_string(path.clone()) {
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                Ok(default_config())
            } else {
                Err(e)?
            }
        }
        Ok(config_contents) => {
            eprintln!("nightwriter: reading configuration from {:#?}", path);
            Ok(toml::from_str(&config_contents)?)
        },
    }
}

fn default_config() -> Config {
    Config {
        output_file_template: None,
        on_start: None,
        on_end: None,
        bindings: None,
    }
}
