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
use crate::ui::util::{create_command_tuple, GRAY, GREEN, RED};
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
        let app = App::new()?;

        let commands_list = CommandsList::create(&get_conf_dir()?);
        let command_logic = app.global::<SlintRustBridge>();

        command_logic.set_commands_list(Self::get_commands_list_data(&commands_list));
        command_logic.set_commands_config(commands_list.to_string().into());
        command_logic.set_command(DEFAULT_COMMAND.to_string().into());
        command_logic.set_deadline(DEFAULT_DEADLINE.to_string().into());
        command_logic.set_ntp(NTP_SYSTEM.to_string().into());

        let bridge = RustSlintBridge {
            app,
            commands_list: Arc::new(Mutex::new(commands_list)),
        };

        bridge.add_on_reset_commands_config();
        bridge.add_on_set_commands_config();
        bridge.add_on_update_application();
        bridge.add_on_add_command();
        bridge.add_on_del_command();
        bridge.add_on_exec_command();
        bridge.add_on_generate_key();

        Ok(bridge)
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
        let ctx = RustSlintBridgeCtx::from_bridge(self);
        self.app.global::<SlintRustBridge>().on_reset_commands_config(move || {
            Self::err_log_wrap(|| {
                info("Resetting commands");
                Self::set_commands_config(ctx.get_cmds_list()?.to_string().into(), &ctx)
            });
        });
    }

    fn set_commands_config(cmds: SharedString, ctx: &RustSlintBridgeCtx) -> Result<(), String> {
        info(&format!("Setting commands:\n{cmds}"));
        ctx.set_cmds(cmds.to_string().lines().map(str::to_string).collect())
    }

    pub fn add_on_set_commands_config(&self) {
        let ctx = RustSlintBridgeCtx::from_bridge(self);
        self.app.global::<SlintRustBridge>().on_set_commands_config(move |cmds| {
            Self::err_log_wrap(|| Self::set_commands_config(cmds, &ctx));
        });
    }

    pub fn add_on_del_command(&self) {
        let ctx = RustSlintBridgeCtx::from_bridge(self);
        self.app.global::<SlintRustBridge>().on_del_command(move |cmd, index| {
            Self::err_log_wrap(|| {
                info(&format!("Removing command: {cmd}"));
                ctx.remove_cmd(cmd, index)
            });
        });
    }

    pub fn add_on_exec_command(&self) {
        let ctx = RustSlintBridgeCtx::from_bridge(self);
        self.app.global::<SlintRustBridge>().on_exec_command(move |cmd, idx, key| {
            Self::err_log_wrap(|| {
                let cmd = cmd.to_string();
                let key = key.to_string();
                let key = key.trim();
                //TODO: Replace naive split_whitespace with proper shell/Clap parsing so quoted
                // arguments survive.
                let mut cmd_vec: Vec<&str> = cmd.split_whitespace().collect();
                cmd_vec.insert(0, "ruroco");

                if !cmd.contains("--key") && !key.is_empty() {
                    cmd_vec.extend(["--key", key]);
                }

                info(&format!("Executing command: {}", cmd_vec.join(" ")));

                let cl = ctx.get_app_cmds_list()?;
                let cl = cl
                    .as_any()
                    .downcast_ref::<VecModel<CommandData>>()
                    .ok_or("Failed to downcast ModelRc to VecModel<CommandData>".to_string())?;
                let cl = cl
                    .as_any()
                    .downcast_ref::<VecModel<CommandData>>()
                    .ok_or("Failed to downcast ModelRc to VecModel<CommandData>".to_string())?;
                match CliClient::try_parse_from(cmd_vec) {
                    Ok(cli_client) => run_client(cli_client)
                        .map(|_| Self::set_command_data_color(idx, &cl, GREEN))
                        .unwrap_or_else(|_| Self::set_command_data_color(idx, &cl, RED)),
                    Err(_) => Self::set_command_data_color(idx, &cl, RED),
                };

                Ok(())
            });
        })
    }

    pub fn add_on_add_command(&self) {
        let ctx = RustSlintBridgeCtx::from_bridge(self);
        self.app.global::<SlintRustBridge>().on_add_command(move |cmd| {
            Self::err_log_wrap(|| {
                info(&format!("Adding new command: {cmd}"));
                ctx.add_cmd(cmd)
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

    #[cfg(target_os = "linux")]
    fn update_linux() -> Result<(), String> {
        Updater::create(false, None, None, false)?.update()
    }

    pub fn add_on_generate_key(&self) {
        self.app
            .global::<SlintRustBridge>()
            .on_generate_key(|| SharedString::from(CryptoHandler::gen_key().unwrap_or_else(|e| e)));
    }

    fn get_commands_list_data(commands_list: &CommandsList) -> ModelRc<CommandData> {
        let commands_list_data: Vec<CommandData> =
            commands_list.get().iter().map(|cmd| create_command_tuple(cmd)).collect();
        ModelRc::from(Rc::new(VecModel::from(commands_list_data)))
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
            .map(|(i, d)| CommandData {
                command: d.command,
                name: d.name,
                color: if i == idx as usize { color } else { GRAY },
            })
            .collect();

        commands_list.set_vec(command_data_vec);
    }
}
