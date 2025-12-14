#[cfg(target_os = "linux")]
use crate::client::update::Updater;
#[cfg(target_os = "android")]
use crate::ui::android_update::update_android;

use crate::client::config::{get_conf_dir, CliClient, DEFAULT_COMMAND};
use crate::client::run_client;
use crate::common::crypto_handler::CryptoHandler;
use crate::common::{error, info};
use crate::ui::colors::{GREEN, RED};
use crate::ui::command_data::{add_command_name, command_to_data, data_to_command};
use crate::ui::rust_slint_bridge_ctx::RustSlintBridgeCtx;
use crate::ui::saved_command_list::CommandsList;
use clap::Parser;
use slint::SharedString;
use std::error::Error;
use std::sync::{Arc, Mutex};

slint::include_modules!();

pub(crate) struct RustSlintBridge {
    pub(crate) app: App,
    pub(crate) commands_list: Arc<Mutex<CommandsList>>,
}

impl RustSlintBridge {
    pub fn create() -> Result<Self, Box<dyn Error>> {
        let bridge = RustSlintBridge {
            app: App::new()?,
            commands_list: Arc::new(Mutex::new(CommandsList::create(&get_conf_dir()?))),
        };
        bridge.set_default_values();
        bridge.add_on_reset_commands_config();
        bridge.add_on_set_commands_config()?;
        bridge.add_on_update_application();
        bridge.add_on_add_command();
        bridge.add_on_del_command();
        bridge.add_on_exec_command();
        bridge.add_on_generate_key();

        Ok(bridge)
    }

    fn set_default_values(&self) {
        let bridge = self.app.global::<SlintRustBridge>();
        bridge.set_command(DEFAULT_COMMAND.to_string().into());
    }

    fn err_log_wrap<F>(msg: String, f: F)
    where
        F: FnOnce() -> Result<(), String>,
    {
        info(&msg);
        if let Err(e) = f() {
            error(&format!("Slint callback failed: {e}"));
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.app.run()
    }

    pub fn add_on_reset_commands_config(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_reset_commands_config(move || {
            Self::err_log_wrap("Resetting commands".to_string(), || ctx.reset_cmds());
        });
    }

    pub fn add_on_set_commands_config(&self) -> Result<(), String> {
        let ctx: RustSlintBridgeCtx = self.into();
        ctx.set_cmds_list()?;
        self.app.global::<SlintRustBridge>().on_set_commands_config(move |cmds| {
            Self::err_log_wrap(format!("Setting commands:\n{cmds}"), || {
                ctx.set_cmds(cmds.split("\n").map(command_to_data).collect())
            });
        });
        Ok(())
    }

    pub fn add_on_del_command(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_del_command(move |cmd| {
            Self::err_log_wrap(format!("Removing command: {cmd:?}"), || ctx.remove_cmd(cmd));
        });
    }

    pub fn add_on_exec_command(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_exec_command(move |cmd, key| {
            Self::err_log_wrap(format!("Executing command: {}", cmd.name), || {
                let key = key.to_string();
                let key = key.trim();
                let cmd_str = data_to_command(&cmd, Some(key.to_string()));
                let mut cmd_vec: Vec<&str> = cmd_str.split_whitespace().collect();
                cmd_vec.insert(0, "ruroco");

                if let Err(e) = CliClient::try_parse_from(cmd_vec)
                    .map_err(|e| e.to_string())
                    .and_then(run_client)
                {
                    error(&format!("Error executing command '{}': {e}", cmd.name));
                    ctx.set_cmd_data_color(&cmd, RED)
                } else {
                    ctx.set_cmd_data_color(&cmd, GREEN)
                }
            });
        })
    }

    pub fn add_on_add_command(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_add_command(move |cmd| {
            Self::err_log_wrap(format!("Adding new command: {cmd:?}"), || {
                ctx.add_cmd(add_command_name(cmd))
            });
        });
    }

    pub fn add_on_update_application(&self) {
        self.app.global::<SlintRustBridge>().on_update_application(move || {
            Self::err_log_wrap("Updating application".to_string(), || {
                #[cfg(target_os = "linux")]
                let update_result = Updater::create(false, None, None, false)?.update();
                #[cfg(target_os = "android")]
                let update_result = update_android();

                update_result
            });
        });
    }

    pub fn add_on_generate_key(&self) {
        self.app
            .global::<SlintRustBridge>()
            .on_generate_key(|| SharedString::from(CryptoHandler::gen_key().unwrap_or_else(|e| e)));
    }
}
