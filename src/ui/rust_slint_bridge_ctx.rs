use crate::ui::rust_slint_bridge::{App, CommandData, RustSlintBridge, SlintRustBridge};
use crate::ui::saved_command_list::CommandsList;
use slint::{ComponentHandle, Model, ModelRc, VecModel, Weak};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct RustSlintBridgeCtx {
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
    pub fn set_cmds(&self, cmds: Vec<CommandData>) -> Result<(), String> {
        let cl = self.get_app_cmds_list()?;
        let cl = cl
            .as_any()
            .downcast_ref::<VecModel<CommandData>>()
            .ok_or("Failed to downcast ModelRc to VecModel<CommandData>".to_string())?;
        cl.set_vec(cmds.clone());

        let mut cl = self.get_cmds_list()?;
        cl.set(cmds);
        Ok(())
    }

    pub fn remove_cmd(&self, cmd: CommandData, index: i32) -> Result<(), String> {
        let cl = self.get_app_cmds_list()?;
        let cl = cl
            .as_any()
            .downcast_ref::<VecModel<CommandData>>()
            .ok_or("Failed to downcast ModelRc to VecModel<CommandData>".to_string())?;
        cl.remove(index as usize);

        let mut cl = self.get_cmds_list()?;
        cl.remove(cmd);
        Ok(())
    }

    pub fn add_cmd(&self, cmd: CommandData) -> Result<(), String> {
        let cl = self.get_app_cmds_list()?;
        let cl = cl
            .as_any()
            .downcast_ref::<VecModel<CommandData>>()
            .ok_or("Failed to downcast ModelRc to VecModel<CommandData>".to_string())?;
        cl.push(cmd.clone());

        let mut cl = self.get_cmds_list()?;
        cl.add(cmd);
        Ok(())
    }

    pub fn get_cmds_list(&'_ self) -> Result<MutexGuard<'_, CommandsList>, String> {
        self.commands_list.lock().map_err(|e| format!("Failed to acquire mutex lock: {e}"))
    }

    pub fn get_app_cmds_list(&self) -> Result<ModelRc<CommandData>, String> {
        let upgraded_app = match self.app.upgrade() {
            Some(a) => a,
            None => return Err("Failed to upgrade weak reference to App".to_string()),
        };
        let commands_list_rc: ModelRc<CommandData> =
            upgraded_app.global::<SlintRustBridge>().get_commands_list();
        Ok(commands_list_rc)
    }
}
