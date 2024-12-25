use std::fs;
use std::path::PathBuf;

use crate::common::{error, resolve_path};
use crate::slint_bridge;
use serde::{Deserialize, Serialize};
use slint::SharedString;
use slint_bridge::CommandTuple;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct CommandsList {
    list: Vec<String>,
    path: PathBuf,
}

impl CommandsList {
    pub fn create(config_dir: &PathBuf) -> CommandsList {
        let commands_list_path = resolve_path(config_dir).join("commands_list.toml");
        let commands_list_str =
            fs::read_to_string(&commands_list_path).unwrap_or_else(|_| "".to_string());
        toml::from_str(&commands_list_str).unwrap_or_else(|_| CommandsList {
            list: vec![],
            path: commands_list_path,
        })
    }

    pub fn create_command_tuple(command: SharedString) -> CommandTuple {
        let command_string: String = command.into();
        CommandsList::create_command_tuple_from_string(&command_string)
    }

    pub fn get(&self) -> Vec<CommandTuple> {
        self.list.iter().map(CommandsList::create_command_tuple_from_string).collect()
    }

    pub fn add(&mut self, command: SharedString) {
        self.list.push(command.into());
        self.save()
    }

    pub fn remove(&mut self, command: SharedString) {
        let entry_str = String::from(command);
        self.list.retain(|value| value != &entry_str);
        self.save()
    }
    fn create_command_tuple_from_string(command: &String) -> CommandTuple {
        CommandTuple {
            command: SharedString::from(command.clone()),
            name: SharedString::from(CommandsList::command_to_name(command)),
        }
    }

    fn command_to_name(command: &String) -> String {
        let arguments: Vec<&str> = command.split_whitespace().filter(|&x| x != "send").collect();
        let mut parts: Vec<String> = arguments
            .iter()
            .enumerate()
            .map(|(idx, val)| match (val, arguments.get(idx + 1)) {
                (val, None) if val.starts_with("--") => {
                    format!("[{}]", val.replace("--", ""))
                }
                (val, Some(next_val)) if val.starts_with("--") && next_val.starts_with("--") => {
                    format!("[{}]", val.replace("--", ""))
                }
                (val, Some(_)) if val.starts_with("--") => {
                    format!(
                        "[{}:",
                        val.replace("--", "").replace("command", "cmd").replace("address", "addr")
                    )
                }
                (_, _) => {
                    format!("{val}]")
                }
            })
            .collect();

        match parts.iter().position(|x| x.contains("private-pem-path")) {
            Some(i) => {
                parts.remove(i); // remove --private-pem-path
                parts.remove(i); // remove the private pem path
            }
            None => {}
        };

        parts.join("").replace("][", "] [")
    }

    fn save(&self) {
        let toml_string = match toml::to_string(&self) {
            Ok(s) => s,
            Err(e) => return error(&format!("Error serializing commands list: {e}")),
        };

        match fs::write(&self.path, toml_string) {
            Ok(_) => (),
            Err(e) => error(&format!("Error persisting commands list: {e}")),
        };
    }
}
