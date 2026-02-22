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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::colors::GRAY;

    fn make_cmd(address: &str, command: &str) -> CommandData {
        CommandData {
            address: address.into(),
            command: command.into(),
            ip: "".into(),
            ipv4: false,
            ipv6: false,
            permissive: false,
            name: "".into(),
            color: GRAY,
        }
    }

    #[test]
    fn test_create_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cl = CommandsList::create(dir.path());
        assert!(cl.list.is_empty());
        assert!(cl.path.to_str().unwrap().contains("commands_list.toml"));
    }

    #[test]
    fn test_add_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        let cmd = make_cmd("host:80", "restart");
        cl.add(cmd);
        let cmds = cl.get();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].address.as_str(), "host:80");
        assert_eq!(cmds[0].command.as_str(), "restart");
    }

    #[test]
    fn test_remove() {
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        let cmd1 = make_cmd("host:80", "restart");
        let cmd2 = make_cmd("host:81", "stop");
        cl.add(cmd1);
        cl.add(cmd2.clone());
        assert_eq!(cl.get().len(), 2);
        cl.remove(cmd2);
        assert_eq!(cl.get().len(), 1);
        assert_eq!(cl.get()[0].address.as_str(), "host:80");
    }

    #[test]
    fn test_set() {
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        cl.add(make_cmd("host:80", "restart"));
        assert_eq!(cl.get().len(), 1);

        let new_cmds = vec![make_cmd("a:1", "x"), make_cmd("b:2", "y")];
        cl.set(new_cmds);
        assert_eq!(cl.get().len(), 2);
    }

    #[test]
    fn test_persistence() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut cl = CommandsList::create(dir.path());
            cl.add(make_cmd("host:80", "restart"));
            cl.add(make_cmd("host:81", "stop"));
        }
        let cl = CommandsList::create(dir.path());
        assert_eq!(cl.get().len(), 2);
    }

    #[test]
    fn test_display() {
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        cl.add(make_cmd("host:80", "restart"));
        let display = format!("{cl}");
        assert!(display.contains("send"));
        assert!(display.contains("host:80"));
    }

    #[test]
    fn test_into_shared_string() {
        let dir = tempfile::tempdir().unwrap();
        let cl = CommandsList::create(dir.path());
        let s: SharedString = (&cl).into();
        assert_eq!(s.as_str(), "");
    }

    #[test]
    fn test_into_model_rc() {
        use slint::Model;
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        cl.add(make_cmd("host:80", "restart"));
        let model: ModelRc<CommandData> = (&cl).into();
        assert_eq!(model.row_count(), 1);
    }

    #[test]
    fn test_sorted_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        cl.add(make_cmd("z:80", "zzz"));
        cl.add(make_cmd("a:80", "aaa"));
        let cmds = cl.get();
        // Commands should be sorted
        let first_addr = cmds[0].address.to_string();
        let second_addr = cmds[1].address.to_string();
        assert!(first_addr <= second_addr, "{first_addr} should be <= {second_addr}");
    }

    #[test]
    fn test_create_with_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commands_list.toml");
        fs::write(&path, "this is {{invalid}} toml").unwrap();
        let cl = CommandsList::create(dir.path());
        assert!(cl.list.is_empty());
    }
}
