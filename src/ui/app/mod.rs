mod dashboard_state;
mod execute_state;

pub(crate) use dashboard_state::{DashboardState, PasteTarget};
pub(crate) use execute_state::{ExecuteState, Status, StatusKey};

use crate::ui::saved_command_list::CommandsList;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Tab {
    Dashboard,
    Create,
    Execute,
}

pub(crate) struct CreateForm {
    pub(crate) address: String,
    pub(crate) command: String,
    pub(crate) ip: String,
    pub(crate) permissive: bool,
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
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
