use crate::mod_keys::ModKeys;
use dirs;
use failure::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub output_file_template: Option<String>,
    pub on_start: Option<BashCommand>,
    pub on_end: Option<BashCommand>,
    pub bindings: Option<Bindings>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub enum Command {
    Bash {
        #[serde(rename = "bash")]
        command: BashCommand,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bindings(pub HashMap<KeyboardShortcut, BashCommand>);

#[derive(Serialize, Deserialize, Debug)]
pub struct BashCommand(pub String);

#[derive(Hash, PartialEq, Eq, Debug, Clone, Arbitrary)]
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
            } else if *chunk == "" {
            } else {
                // FIXME: proper error
                panic!(
                    "Unexpected modifier (expected C for Ctrl, S for Shift, or M for Alt): {}",
                    chunk
                );
            }
        }
        let last_chunk = chunks.last().unwrap_or(&"");
        let key = if last_chunk.len() != 1 {
            if chunks.len() > 1 && chunks[chunks.len() - 2] == "" {
                '-'
            } else {
                // FIXME: proper error
                panic!(
                    "Expected keyboard shortcut key to be just one ascii char, instad got: {}",
                    last_chunk
                )
            }
        } else {
            last_chunk.chars().nth(0).unwrap()
        };
        return Ok(KeyboardShortcut {
            mod_keys: ModKeys { ctrl, shift },
            key,
        });
    }
}

impl Serialize for KeyboardShortcut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut result = String::with_capacity(6);
        if self.mod_keys.ctrl {
            result.push_str("C-");
        }
        if self.mod_keys.shift {
            result.push_str("S-");
        }
        result.push(self.key);
        result.serialize(serializer)
    }
}

pub fn read_config(opt_config_path: Option<PathBuf>) -> Result<Config, Error> {
    let path = match opt_config_path {
        Some(path) => path,
        None => match dirs::config_dir() {
            None => {
                eprintln!("nightwriter: Neither XDG_CONFIG_HOME nor HOME environment variables are set, or --config option, so using default configuration");
                return Ok(default_config());
            }
            Some(dir) => {
                let mut path = dir.clone();
                path.push("nightwriter");
                path.push("config.toml");
                path
            }
        },
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
    struct Wrapper<T> {
        field: T,
    }

    quickcheck! {
        fn keyboard_shortcut_roundtrips(input: KeyboardShortcut) -> bool {
            if !input.key.is_ascii() || !input.key.is_alphanumeric() {
                // TODO: ideally not meeting a precondition would not count as a test of the
                // property.
                return true;
            } else {
                eprintln!("input = {:?}", input);
                match toml::to_string(&Wrapper { field: input.clone() }) {
                    Ok(serialized) => {
                        eprintln!("serialized = {}", serialized);
                        let result = toml::from_str(&serialized);
                        eprintln!("deserialized = {:?}",  result);
                        Ok(Wrapper { field: input }) == result
                    }
                    Err(_) => false,
                }
            }
        }
    }
}
