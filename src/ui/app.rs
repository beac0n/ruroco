use crate::common::logging::error;
use crate::ui::colors::GRAY;
use crate::ui::command_data::CommandData;
use crate::ui::saved_command_list::CommandsList;
use eframe::egui;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum PasteTarget {
    Key,
    Config,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tab {
    Dashboard,
    Create,
    Execute,
}

#[derive(Clone, Copy, PartialEq, Debug)]
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

pub(crate) struct DashboardState {
    pub(crate) config_text: String,
    pub(crate) key: String,
    pub(crate) show_key: bool,
    pub(crate) paste_target: Option<PasteTarget>,
}

#[cfg(target_os = "android")]
const KEY_PREF: &str = "aes_key";

impl DashboardState {
    pub(crate) fn load_persisted_key() -> String {
        #[cfg(target_os = "android")]
        {
            match crate::common::android::AndroidPrefs::get_string(KEY_PREF) {
                Ok(Some(k)) => k,
                Ok(None) => String::new(),
                Err(e) => {
                    error(format!("Failed to load AES key: {e}"));
                    String::new()
                }
            }
        }
        #[cfg(not(target_os = "android"))]
        String::new()
    }

    #[allow(unused_variables)]
    pub(crate) fn save_key(&mut self, key: String) {
        self.key = key;
        #[cfg(target_os = "android")]
        if let Err(e) = crate::common::android::AndroidPrefs::put_string(KEY_PREF, &self.key) {
            error(format!("Failed to save AES key: {e}"));
        }
    }
}

pub(crate) struct CreateForm {
    pub(crate) address: String,
    pub(crate) command: String,
    pub(crate) ip: String,
    pub(crate) permissive: bool,
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
}

pub(crate) struct ExecuteState {
    pub(crate) status: HashMap<StatusKey, Status>,
}

impl ExecuteState {
    pub(crate) fn color_for(&self, cmd: &CommandData) -> egui::Color32 {
        use crate::ui::colors::{GREEN, RED};
        match self.status.get(&StatusKey::from(cmd)) {
            Some(Status::Ok) => GREEN,
            Some(Status::Err) => RED,
            _ => GRAY,
        }
    }

    pub(crate) fn set(&mut self, cmd: &CommandData, status: Status) {
        self.status.insert(StatusKey::from(cmd), status);
    }
}

pub(crate) struct RurocoApp {
    pub(crate) commands_list: CommandsList,
    pub(crate) active_tab: Tab,
    pub(crate) status_bar_dp: f32,
    pub(crate) dashboard: DashboardState,
    pub(crate) create: CreateForm,
    pub(crate) execute: ExecuteState,
}

impl RurocoApp {
    pub(crate) fn new(conf_dir: &Path) -> anyhow::Result<Self> {
        Self::new_with_status_bar(conf_dir, 0.0)
    }

    pub(crate) fn new_with_status_bar(conf_dir: &Path, status_bar_dp: f32) -> anyhow::Result<Self> {
        use crate::client::config::DEFAULT_COMMAND;
        let commands_list = CommandsList::create(conf_dir);
        let config_text = commands_list.to_string();
        Ok(Self {
            commands_list,
            active_tab: Tab::Dashboard,
            status_bar_dp,
            dashboard: DashboardState {
                config_text,
                key: DashboardState::load_persisted_key(),
                show_key: false,
                paste_target: None,
            },
            create: CreateForm {
                address: String::new(),
                command: DEFAULT_COMMAND.to_string(),
                ip: String::new(),
                permissive: false,
                ipv4: false,
                ipv6: false,
            },
            execute: ExecuteState {
                status: HashMap::new(),
            },
        })
    }
}
