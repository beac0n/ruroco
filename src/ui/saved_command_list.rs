use crate::common::fs::write_atomic;
use crate::common::logging::error;
use crate::common::resolve_path;
use crate::ui::command_data::{add_command_name, command_to_data, data_to_command, CommandData};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CommandsList {
    list: Vec<CommandData>,
    #[serde(skip)]
    path: PathBuf,
}

impl fmt::Display for CommandsList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, c) in self.list.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", data_to_command(c, None))?;
        }
        Ok(())
    }
}

impl CommandsList {
    pub(crate) fn create(cfg_dir: &Path) -> CommandsList {
        let path = resolve_path(cfg_dir).join("commands_list.toml");
        let raw = CommandsList::read_raw_from_path(&path);

        let mut cmd_list = toml::from_str::<CommandsList>(&raw)
            .or_else(|_| {
                // Legacy format: list was Vec<String> of CLI invocations
                #[derive(Deserialize)]
                struct Legacy {
                    list: Vec<String>,
                }
                toml::from_str::<Legacy>(&raw).map(|l| CommandsList {
                    list: l.list.into_iter().map(|s| command_to_data(&s)).collect(),
                    path: PathBuf::new(),
                })
            })
            .unwrap_or_else(|e| {
                if !raw.is_empty() {
                    error(format!(
                        "commands_list.toml parse failed, starting empty (file preserved): {e}"
                    ));
                }
                CommandsList {
                    list: vec![],
                    path: PathBuf::new(),
                }
            });

        cmd_list.path = path;
        cmd_list.list = cmd_list.list.into_iter().map(add_command_name).collect();
        cmd_list.sort();
        cmd_list
    }

    pub(crate) fn get(&self) -> &[CommandData] {
        &self.list
    }

    pub(crate) fn add(&mut self, cmd: CommandData) {
        self.list.push(cmd);
        self.sort();
        self.save();
    }

    pub(crate) fn set(&mut self, list: Vec<CommandData>) {
        self.list = list;
        self.sort();
        self.save();
    }

    pub(crate) fn remove(&mut self, cmd: &CommandData) {
        self.list.retain(|c| c != cmd);
        self.save();
    }

    fn sort(&mut self) {
        self.list.sort_by(|a, b| (&a.command, &a.address).cmp(&(&b.command, &b.address)));
    }

    fn read_raw_from_path(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|_| String::new())
    }

    fn save(&self) {
        let toml_str = match toml::to_string(&self) {
            Ok(s) => s,
            Err(e) => return error(format!("Error serializing commands list: {e}")),
        };

        match write_atomic(&self.path, toml_str.as_bytes()) {
            Ok(_) => (),
            Err(e) => error(format!("Error persisting commands list: {e}")),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cmd(address: &str, command: &str) -> CommandData {
        CommandData {
            address: address.to_string(),
            command: command.to_string(),
            ip: String::new(),
            ipv4: false,
            ipv6: false,
            permissive: false,
            name: String::new(),
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
        assert_eq!(cmds[0].address, "host:80");
        assert_eq!(cmds[0].command, "restart");
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
        cl.remove(&cmd2);
        assert_eq!(cl.get().len(), 1);
        assert_eq!(cl.get()[0].address, "host:80");
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
    fn test_sorted_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut cl = CommandsList::create(dir.path());
        cl.add(make_cmd("z:80", "zzz"));
        cl.add(make_cmd("a:80", "aaa"));
        let cmds = cl.get();
        let first_addr = cmds[0].address.clone();
        let second_addr = cmds[1].address.clone();
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

    #[test]
    fn test_legacy_format_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commands_list.toml");
        // Write old-format file (list of CLI strings)
        fs::write(
            &path,
            "list = [\"send --address host:80 --command restart\", \"send --address host:81 --command stop\"]\n",
        )
        .unwrap();
        let cl = CommandsList::create(dir.path());
        assert_eq!(cl.get().len(), 2);
        let addrs: Vec<&str> = cl.get().iter().map(|c| c.address.as_str()).collect();
        assert!(addrs.contains(&"host:80"));
        assert!(addrs.contains(&"host:81"));
    }
}
