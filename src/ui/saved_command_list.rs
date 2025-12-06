use crate::common::{error, resolve_path};
use crate::ui::command_data::{command_to_data, data_to_command};
use crate::ui::rust_slint_bridge::CommandData;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
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

    pub fn set(&mut self, commands_list: Vec<CommandData>) {
        self.list = commands_list.into_iter().map(|c| data_to_command(&c, None)).collect();
        self.save()
    }

    pub fn get(&self) -> Vec<CommandData> {
        self.list.clone().into_iter().map(|c| command_to_data(&c)).collect()
    }

    pub fn add(&mut self, command: CommandData) {
        self.list.push(data_to_command(&command, None));
        self.save()
    }

    pub fn remove(&mut self, command: CommandData) {
        let cmd_str = data_to_command(&command, None);
        self.list.retain(|value| value.clone() != cmd_str.clone());
        self.save()
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
