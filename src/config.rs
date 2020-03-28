use crate::mod_keys::ModKeys;
use dirs;
use failure::Error;
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::string::String;
use std::vec::Vec;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub output_file_template: Option<String>,
    pub on_start: Option<Command>,
    pub on_end: Option<Command>,
    pub bindings: Option<Bindings>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bindings(HashMap<KeyboardShortcut, Vec<Command>>);

#[derive(Hash, PartialEq, Eq, Debug, Clone, Arbitrary)]
pub enum Command {
    Bash(String),
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Arbitrary)]
pub struct KeyboardShortcut {
    pub mod_keys: ModKeys,
    pub key: char,
}

/*
pub fn output_test_config() {
    let mut bindings = HashMap::new();
    let mut commands = Vec::new();
    commands.push(Command::Bash(String::from("echo hi")));
    bindings.insert(
        KeyboardShortcut {
            mod_keys: ModKeys {
                ctrl: true,
                shift: false,
            },
            key: 'c',
        },
        commands,
    );
    eprintln!(
        "{}",
        toml::to_string(&Config {
            bindings: Some(Bindings(bindings)),
            on_end: None,
            on_start: None,
            output_file_template: None
        })
        .unwrap()
    );
}
*/

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

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Command, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(CommandVisitor)
    }
}

struct CommandVisitor;

impl<'de> de::Visitor<'de> for CommandVisitor {
    type Value = Command;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        // TODO: improve
        formatter.write_str("a command")
    }

    fn visit_str<E>(self, input: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let prefixes = vec!["bash"];
        let input_vec: Vec<&str> = input.splitn(2, ' ').collect();
        match input_vec.as_slice() {
            [prefix, command] if *prefix == "bash" => Ok(Command::Bash(String::from(*command))),
            [_, _] | [_] => Err(E::custom(format!(
                "Unexpected command prefix in command {:?} (expected one of {:?})",
                input, prefixes
            ))),
            _ => panic!("Impossible case"),
        }
    }
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (match self {
            Command::Bash(command) => format!("bash {}", command),
        })
        .serialize(serializer)
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
    /* Re-enable once toml-rs#372 is included in a release

    use super::*;
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
    struct Wrapper<T> {
        field: T,
    }

    fn check_roundtrip<T: Clone + Debug + Serialize + DeserializeOwned + PartialEq>(
        input: T,
    ) -> bool {
        eprintln!("input = {:?}", input);
        match toml::to_string(&Wrapper {
            field: input.clone(),
        }) {
            Ok(serialized) => {
                eprintln!("serialized = {}", serialized);
                let result = toml::from_str(&serialized);
                eprintln!("deserialized = {:?}", result);
                Ok(Wrapper { field: input }) == result
            }
            Err(_) => false,
        }
    }

    quickcheck! {
        fn keyboard_shortcut_roundtrips(input: KeyboardShortcut) -> bool {
            if !input.key.is_ascii() || !input.key.is_alphanumeric() {
                // TODO: ideally not meeting a precondition would not count as a test of the
                // property.
                return true;
            } else {
                check_roundtrip(input)
            }
        }

        fn command_roundtrips(input: Command) -> bool {
            check_roundtrip(input)
        }

        fn string_roundtrips(input: String) -> bool {
            check_roundtrip(input)
        }
    }

    */
}
