use crate::ui::colors::GRAY;
use crate::ui::command_data::CommandData;
use crate::ui::saved_command_list::CommandsList;
use eframe::egui;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PasteTarget {
    Key,
    Config,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Tab {
    Dashboard,
    Create,
    Execute,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Status {
    Ok,
    Err,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub(crate) struct StatusKey {
    command: String,
    address: String,
    ip: String,
    ipv4: bool,
    ipv6: bool,
    permissive: bool,
}

impl From<&CommandData> for StatusKey {
    fn from(c: &CommandData) -> Self {
        Self {
            command: c.command.clone(),
            address: c.address.clone(),
            ip: c.ip.clone(),
            ipv4: c.ipv4,
            ipv6: c.ipv6,
            permissive: c.permissive,
        }
    }
}

pub(crate) struct RurocoApp {
    pub(crate) commands_list: CommandsList,
    pub(crate) commands_config_text: String,
    pub(crate) key: String,
    pub(crate) command: String,
    pub(crate) show_key: bool,
    pub(crate) paste_target: Option<PasteTarget>,
    pub(crate) active_tab: Tab,
    pub(crate) create_address: String,
    pub(crate) create_ip: String,
    pub(crate) create_permissive: bool,
    pub(crate) create_ipv4: bool,
    pub(crate) create_ipv6: bool,
    pub(crate) command_status: HashMap<StatusKey, Status>,
    pub(crate) cached_commands: Vec<CommandData>,
    pub(crate) status_bar_dp: f32,
}

impl RurocoApp {
    pub(crate) fn new(conf_dir: &Path) -> anyhow::Result<Self> {
        Self::new_with_status_bar(conf_dir, 0.0)
    }

    pub(crate) fn new_with_status_bar(conf_dir: &Path, status_bar_dp: f32) -> anyhow::Result<Self> {
        use crate::client::config::DEFAULT_COMMAND;
        let commands_list = CommandsList::create(conf_dir);
        let commands_config_text = commands_list.to_string();
        let cached_commands = commands_list.get().to_vec();
        Ok(Self {
            commands_list,
            commands_config_text,
            key: String::new(),
            command: DEFAULT_COMMAND.to_string(),
            show_key: false,
            paste_target: None,
            active_tab: Tab::Dashboard,
            create_address: String::new(),
            create_ip: String::new(),
            create_permissive: false,
            create_ipv4: false,
            create_ipv6: false,
            command_status: HashMap::new(),
            cached_commands,
            status_bar_dp,
        })
    }

    pub(crate) fn refresh_cache(&mut self) {
        self.cached_commands = self.commands_list.get().to_vec();
        self.commands_config_text = self.commands_list.to_string();
    }

    pub(crate) fn status_color(&self, cmd: &CommandData) -> egui::Color32 {
        use crate::ui::colors::{GREEN, RED};
        match self.command_status.get(&StatusKey::from(cmd)) {
            Some(Status::Ok) => GREEN,
            Some(Status::Err) => RED,
            _ => GRAY,
        }
    }

    pub(crate) fn set_status(&mut self, cmd: &CommandData, status: Status) {
        self.command_status.insert(StatusKey::from(cmd), status);
    }
}
