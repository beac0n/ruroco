use crate::ui::colors::{change_color, GRAY};
use crate::ui::rust_slint_bridge::{App, CommandData, RustSlintBridge, SlintRustBridge};
use crate::ui::saved_command_list::CommandsList;
use anyhow::anyhow;
use slint::{Color, ComponentHandle, Model, VecModel, Weak};
use std::sync::{Arc, Mutex, MutexGuard};

pub(crate) struct RustSlintBridgeCtx {
    app: Weak<App>,
    commands_list: Arc<Mutex<CommandsList>>,
}

impl From<&RustSlintBridge> for RustSlintBridgeCtx {
    fn from(bridge: &RustSlintBridge) -> Self {
        Self {
            app: bridge.app.as_weak(),
            commands_list: Arc::clone(&bridge.commands_list),
        }
    }
}

impl RustSlintBridgeCtx {
    pub(crate) fn set_cmd_data_color(&self, cmd: &CommandData, color: Color) -> anyhow::Result<()> {
        let app = self.get_upgraded_app()?;
        let cl = app.global::<SlintRustBridge>().get_commands_list();
        let cl = cl
            .as_any()
            .downcast_ref::<VecModel<CommandData>>()
            .ok_or_else(|| anyhow!("Failed to downcast ModelRc to VecModel<CommandData>"))?;

        let cmd_data_vec: Vec<CommandData> = cl
            .iter()
            .map(|d| {
                let color = if &d == cmd { color } else { GRAY };
                change_color(d, color)
            })
            .collect();

        cl.set_vec(cmd_data_vec);
        Ok(())
    }

    pub(crate) fn set_cmds_list(&self) -> anyhow::Result<()> {
        let cmds_list = self.get_cmds_list()?;
        let app = self.get_upgraded_app()?;
        app.global::<SlintRustBridge>().set_commands_config((&*cmds_list).into());
        app.global::<SlintRustBridge>().set_commands_list((&*cmds_list).into());
        Ok(())
    }

    pub(crate) fn reset_cmds(&self) -> anyhow::Result<()> {
        self.set_cmds(self.get_cmds_list()?.get())
    }
    pub(crate) fn set_cmds(&self, cmds: Vec<CommandData>) -> anyhow::Result<()> {
        let mut cl = self.get_cmds_list()?;
        cl.set(cmds);
        self.set_cmds_list()
    }

    pub(crate) fn remove_cmd(&self, cmd: CommandData) -> anyhow::Result<()> {
        let mut cl = self.get_cmds_list()?;
        cl.remove(cmd);
        self.set_cmds_list()
    }

    pub(crate) fn add_cmd(&self, cmd: CommandData) -> anyhow::Result<()> {
        let mut cl = self.get_cmds_list()?;
        cl.add(cmd);
        self.set_cmds_list()
    }

    fn get_cmds_list(&'_ self) -> anyhow::Result<MutexGuard<'_, CommandsList>> {
        self.commands_list.lock().map_err(|e| anyhow!("Failed to acquire mutex lock: {e}"))
    }

    fn get_upgraded_app(&self) -> anyhow::Result<App> {
        self.app.upgrade().ok_or_else(|| anyhow!("Failed to upgrade weak reference to App"))
    }
}
