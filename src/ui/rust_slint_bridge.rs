#[cfg(target_os = "android")]
use crate::ui::android_util::AndroidUtil;

use crate::client::config::{get_conf_dir, CliClient, DEFAULT_COMMAND, DEFAULT_DEADLINE};
use crate::client::run_client;
use crate::client::update::Updater;
use crate::common::crypto_handler::CryptoHandler;
use crate::common::time_util::NTP_SYSTEM;
use crate::common::{error, info};
use crate::ui::rust_slint_bridge_ctx::RustSlintBridgeCtx;
use crate::ui::saved_command_list::CommandsList;
use crate::ui::util::{
    add_command_name, change_color, command_to_data, data_to_command, GRAY, GREEN, RED,
};
use clap::Parser;
use slint::{Color, Model, ModelRc, SharedString, VecModel};
use std::error::Error;
use std::rc::Rc;
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
        bridge.set_deadline(DEFAULT_DEADLINE.to_string().into());
        bridge.set_ntp(NTP_SYSTEM.to_string().into());
    }

    fn err_log_wrap<F>(f: F)
    where
        F: FnOnce() -> Result<(), String>,
    {
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
            Self::err_log_wrap(|| {
                info("Resetting commands");
                ctx.set_cmds(ctx.get_cmds_list()?.get())
            });
        });
    }

    pub fn add_on_set_commands_config(&self) -> Result<(), String> {
        let bridge = self.app.global::<SlintRustBridge>();
        let ctx: RustSlintBridgeCtx = self.into();
        let commands_list = ctx.get_cmds_list()?;
        bridge.set_commands_config(commands_list.to_string().into());
        bridge.set_commands_list(Self::get_commands_list_data(&commands_list));

        let ctx: RustSlintBridgeCtx = self.into();
        bridge.on_set_commands_config(move |cmds| {
            Self::err_log_wrap(|| {
                info(&format!("Setting commands:\n{cmds}"));
                let cmds_vec: Vec<CommandData> = cmds.split("\n").map(command_to_data).collect();
                ctx.set_cmds(cmds_vec)
            });
        });
        Ok(())
    }

    pub fn add_on_del_command(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_del_command(move |cmd, index| {
            Self::err_log_wrap(|| {
                info(&format!("Removing command: {cmd:?}"));
                ctx.remove_cmd(cmd, index)
            });
        });
    }

    pub fn add_on_exec_command(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_exec_command(move |cmd, idx, key| {
            Self::err_log_wrap(|| {
                let key = key.to_string();
                let key = key.trim();

                let cmd_str = data_to_command(&cmd, Some(key.to_string()));
                let mut cmd_vec: Vec<&str> = cmd_str.split_whitespace().collect();
                cmd_vec.insert(0, "ruroco");
                info(&format!("Executing command: {}", cmd.name));

                let cl = ctx.get_app_cmds_list()?;
                let cl = cl
                    .as_any()
                    .downcast_ref::<VecModel<CommandData>>()
                    .ok_or("Failed to downcast ModelRc to VecModel<CommandData>".to_string())?;
                match CliClient::try_parse_from(cmd_vec) {
                    Ok(cli_client) => run_client(cli_client)
                        .map(|_| Self::set_command_data_color(idx, cl, GREEN))
                        .unwrap_or_else(|_| Self::set_command_data_color(idx, cl, RED)),
                    Err(_) => Self::set_command_data_color(idx, cl, RED),
                };

                Ok(())
            });
        })
    }

    pub fn add_on_add_command(&self) {
        let ctx: RustSlintBridgeCtx = self.into();
        self.app.global::<SlintRustBridge>().on_add_command(move |cmd| {
            Self::err_log_wrap(|| {
                info(&format!("Adding new command: {cmd:?}"));
                ctx.add_cmd(add_command_name(cmd))
            });
        });
    }

    pub fn add_on_update_application(&self) {
        self.app.global::<SlintRustBridge>().on_update_application(move || {
            Self::err_log_wrap(|| {
                #[cfg(target_os = "linux")]
                let update_result = Self::update_linux();
                #[cfg(target_os = "android")]
                let update_result = Self::update_android();

                update_result
            });
        });
    }

    pub fn add_on_generate_key(&self) {
        self.app
            .global::<SlintRustBridge>()
            .on_generate_key(|| SharedString::from(CryptoHandler::gen_key().unwrap_or_else(|e| e)));
    }

    fn get_commands_list_data(commands_list: &CommandsList) -> ModelRc<CommandData> {
        let commands_list_data: Vec<CommandData> = commands_list.get();
        ModelRc::from(Rc::new(VecModel::from(commands_list_data)))
    }

    #[cfg(target_os = "linux")]
    fn update_linux() -> Result<(), String> {
        Updater::create(false, None, None, false)?.update()
    }

    #[cfg(target_os = "android")]
    fn update_android() -> Result<(), String> {
        let data = Updater::get_github_api_data(None)?;
        let asset = data
            .assets
            .into_iter()
            .find(|a| a.name.ends_with(".apk"))
            .ok_or(Err("No APK asset found in latest release"))?;

        let util = AndroidUtil::create()?;
        let uri = util.uri_parse(asset.browser_download_url)?;
        let intent = util.new_view_intent(&uri)?;
        let result = util.start_activity(&intent);
        let _ = result.inspect_err(|err| {
            info(&format!("Error (prob. expected) when opening browser window: {err}"))
        });

        Ok(())
    }

    fn set_command_data_color(idx: i32, commands_list: &VecModel<CommandData>, color: Color) {
        let command_data_vec: Vec<CommandData> = commands_list
            .iter()
            .enumerate()
            .map(|(i, d)| change_color(d, if i == idx as usize { color } else { GRAY }))
            .collect();

        commands_list.set_vec(command_data_vec);
    }
}
