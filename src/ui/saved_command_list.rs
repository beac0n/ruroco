use crate::common::{error, resolve_path};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct CommandsList {
    list: Vec<String>,
    path: PathBuf,
}

impl fmt::Display for CommandsList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.list.join("\n"))
    }
}

impl CommandsList {
    pub fn create(config_dir: &Path) -> CommandsList {
        let commands_list_path = resolve_path(config_dir).join("commands_list.toml");
        let commands_list_str = CommandsList::read_raw_from_path(&commands_list_path);
        toml::from_str(&commands_list_str).unwrap_or_else(|_| CommandsList {
            list: vec![],
            path: commands_list_path,
        })
    }

    pub fn set(&mut self, commands_list: Vec<String>) {
        self.list = commands_list;
        self.save()
    }

    pub fn get(&self) -> Vec<String> {
        self.list.clone()
    }

    pub fn add(&mut self, command: String) {
        self.list.push(command);
        self.save()
    }

    pub fn remove(&mut self, command: String) {
        self.list.retain(|value| value != &command);
        self.save()
    }

    pub fn command_to_name(command: &str) -> String {
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

        if let Some(i) = parts.iter().position(|x| x.contains("private-pem-path")) {
            parts.remove(i); // remove --private-pem-path
            parts.remove(i); // remove the private pem path
        };

        parts.join("")
    }

    fn read_raw_from_path(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|_| "".to_string())
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
