use crate::ui::colors::{GRAY, GREEN, RED};
use crate::ui::command_data::CommandData;
use eframe::egui;
use std::collections::HashMap;

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

pub(crate) struct ExecuteState {
    pub(crate) status: HashMap<StatusKey, Status>,
}

impl ExecuteState {
    pub(crate) fn color_for(&self, cmd: &CommandData) -> egui::Color32 {
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
