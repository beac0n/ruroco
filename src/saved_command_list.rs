use std::fs;
use std::path::PathBuf;

use crate::common::{error, resolve_path};
use serde::{Deserialize, Serialize};
use slint::SharedString;

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

    pub fn get(&self) -> Vec<SharedString> {
        self.list.iter().map(SharedString::from).collect()
    }

    pub fn add(&mut self, entry: SharedString) {
        self.list.push(entry.into());
        self.save()
    }

    pub fn remove(&mut self, entry: SharedString) {
        let entry_str = String::from(entry);
        self.list.retain(|value| value != &entry_str);
        self.save()
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
