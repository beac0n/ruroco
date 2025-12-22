use crate::common::{error, resolve_path};
use crate::ui::command_data::{command_to_data, data_to_command};
use crate::ui::rust_slint_bridge::CommandData;
use serde::{Deserialize, Serialize};
use slint::{ModelRc, SharedString, VecModel};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fmt, fs};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CommandsList {
    list: Vec<String>,
    path: PathBuf,
}

impl fmt::Display for CommandsList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.list.join("\n"))
    }
}

impl From<&CommandsList> for SharedString {
    fn from(cl: &CommandsList) -> Self {
        SharedString::from(cl.to_string())
    }
}

impl From<&CommandsList> for ModelRc<CommandData> {
    fn from(cl: &CommandsList) -> Self {
        ModelRc::from(Rc::new(VecModel::from(cl.get())))
    }
}

impl CommandsList {
    pub(crate) fn create(cfg_dir: &Path) -> CommandsList {
        let path = resolve_path(cfg_dir).join("commands_list.toml");
        let mut cmd_list = toml::from_str(&CommandsList::read_raw_from_path(&path))
            .unwrap_or_else(|_| CommandsList { list: vec![], path });
        cmd_list.list.sort();
        cmd_list
    }

    pub(crate) fn set(&mut self, cmd_list: Vec<CommandData>) {
        self.list = cmd_list.into_iter().map(|c| data_to_command(&c, None)).collect();
        self.list.sort();
        self.save()
    }

    pub(crate) fn get(&self) -> Vec<CommandData> {
        self.list.clone().into_iter().map(|c| command_to_data(&c)).collect()
    }

    pub(crate) fn add(&mut self, cmd: CommandData) {
        self.list.push(data_to_command(&cmd, None));
        self.list.sort();
        self.save()
    }

    pub(crate) fn remove(&mut self, cmd: CommandData) {
        let cmd_str = data_to_command(&cmd, None);
        self.list.retain(|value| value.clone() != cmd_str.clone());
        self.save()
    }

    fn read_raw_from_path(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|_| "".to_string())
    }

    fn save(&self) {
        let toml_str = match toml::to_string(&self) {
            Ok(s) => s,
            Err(e) => return error(format!("Error serializing commands list: {e}")),
        };

        match fs::write(&self.path, toml_str) {
            Ok(_) => (),
            Err(e) => error(format!("Error persisting commands list: {e}")),
        };
    }
}
