use crate::ui::colors::GRAY;
use crate::ui::command_data::CommandData;
use crate::ui::saved_command_list::CommandsList;
use crate::ui::tabs;
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
    pub(crate) command_status: HashMap<String, Status>,
    pub(crate) cached_commands: Vec<CommandData>,
    status_bar_dp: f32,
}

impl RurocoApp {
    pub(crate) fn new(conf_dir: &Path) -> anyhow::Result<Self> {
        Self::new_with_status_bar(conf_dir, 0.0)
    }

    pub(crate) fn new_with_status_bar(conf_dir: &Path, status_bar_dp: f32) -> anyhow::Result<Self> {
        use crate::client::config::DEFAULT_COMMAND;
        let commands_list = CommandsList::create(conf_dir);
        let commands_config_text = commands_list.to_string();
        let cached_commands = commands_list.get();
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
        self.cached_commands = self.commands_list.get();
        self.commands_config_text = self.commands_list.to_string();
    }

    pub(crate) fn status_color(&self, cmd: &CommandData) -> egui::Color32 {
        use crate::ui::colors::{GREEN, RED};
        let key = crate::ui::command_data::data_to_command(cmd, None);
        match self.command_status.get(&key) {
            Some(Status::Ok) => GREEN,
            Some(Status::Err) => RED,
            _ => GRAY,
        }
    }

    pub(crate) fn set_status(&mut self, cmd: &CommandData, status: Status) {
        let key = crate::ui::command_data::data_to_command(cmd, None);
        self.command_status.insert(key, status);
    }
}

impl eframe::App for RurocoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.status_bar_dp > 0.0 {
            ui.add_space(self.status_bar_dp);
        }

        #[cfg(all(target_os = "android", feature = "android-build"))]
        if ui.ctx().wants_keyboard_input() {
            let _ = crate::common::android_util::show_soft_keyboard()
                .inspect_err(|e| crate::common::logging::error(format!("{e}")));
        } else {
            let _ = crate::common::android_util::hide_soft_keyboard()
                .inspect_err(|e| crate::common::logging::error(format!("{e}")));
        }

        egui::Panel::top("tabs").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Dashboard, "Dashboard");
                ui.selectable_value(&mut self.active_tab, Tab::Create, "Create");
                ui.selectable_value(&mut self.active_tab, Tab::Execute, "Execute");
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| match self.active_tab {
            Tab::Dashboard => tabs::dashboard::render(self, ui),
            Tab::Create => tabs::create::render(self, ui),
            Tab::Execute => tabs::execute::render(self, ui),
        });
    }
}
