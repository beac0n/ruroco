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

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use crate::ui::colors::{GRAY, GREEN, RED};

    fn sample() -> CommandData {
        CommandData {
            name: "t".into(),
            address: "127.0.0.1:80".into(),
            command: "default".into(),
            ip: String::new(),
            permissive: false,
            ipv4: false,
            ipv6: false,
        }
    }

    #[test]
    fn test_color_gray_when_absent() {
        let state = ExecuteState {
            status: HashMap::new(),
        };
        assert_eq!(state.color_for(&sample()), GRAY);
    }

    #[test]
    fn test_color_green_on_ok() {
        let mut state = ExecuteState {
            status: HashMap::new(),
        };
        let cmd = sample();
        state.set(&cmd, Status::Ok);
        assert_eq!(state.color_for(&cmd), GREEN);
    }

    #[test]
    fn test_color_red_on_err() {
        let mut state = ExecuteState {
            status: HashMap::new(),
        };
        let cmd = sample();
        state.set(&cmd, Status::Err);
        assert_eq!(state.color_for(&cmd), RED);
    }
}
