#[cfg(target_os = "android")]
use crate::ui::android_util::AndroidUtil;

use crate::client::run_client;
use crate::client::update::Updater;
use crate::common::{error, info, NTP_SYSTEM};
use crate::config::config_client::{get_conf_dir, CliClient, DEFAULT_COMMAND, DEFAULT_DEADLINE};
use crate::ui::saved_command_list::CommandsList;
use clap::Parser;
use slint::{Color, Model, ModelRc, SharedString, VecModel, Weak};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

slint::include_modules!();

const GREEN: Color = Color::from_rgb_u8(56, 142, 60);
const RED: Color = Color::from_rgb_u8(211, 47, 47);
const GRAY: Color = Color::from_rgb_u8(204, 204, 204);

pub struct RustSlintBridge {
    app: App,
    commands_list: Arc<Mutex<CommandsList>>,
    private_pem_path: String,
    public_pem_path: String,
}

#[derive(Clone)]
pub struct RustSlintBridgeExecutor {
    app: Weak<App>,
    public_pem_path: String,
}

impl RustSlintBridgeExecutor {
    pub fn enable_key_gen_popup(&self) {
        self.app.unwrap().global::<SlintRustBridge>().set_generating_keys(true)
    }

    pub fn disable_key_gen_popup(&self) {
        self.app.unwrap().global::<SlintRustBridge>().set_generating_keys(false)
    }

    pub fn set_public_key(&self) -> Result<(), Box<dyn Error>> {
        self.app
            .unwrap()
            .global::<SlintRustBridge>()
            .set_public_key(fs::read_to_string(&self.public_pem_path)?.into());
        Ok(())
    }
}

impl RustSlintBridge {
    pub fn create(public_pem_path: &Path, private_pem_path: &Path) -> Result<Self, Box<dyn Error>> {
        let app = App::new()?;

        let commands_list = CommandsList::create(&get_conf_dir());
        let command_logic = app.global::<SlintRustBridge>();

        command_logic.set_commands_list(Self::get_commands_list_data(&commands_list));
        command_logic.set_commands_config(commands_list.to_string().into());
        command_logic.set_command(DEFAULT_COMMAND.to_string().into());
        command_logic.set_deadline(DEFAULT_DEADLINE.to_string().into());
        command_logic.set_ntp(NTP_SYSTEM.to_string().into());

        Ok(RustSlintBridge {
            app,
            commands_list: Arc::new(Mutex::new(commands_list)),
            public_pem_path: public_pem_path
                .to_str()
                .ok_or("Could not convert public pem path to string")?
                .to_string(),
            private_pem_path: private_pem_path
                .to_str()
                .ok_or("Could not convert private pem path to string")?
                .to_string(),
        })
    }

    pub fn create_executor(&self) -> RustSlintBridgeExecutor {
        RustSlintBridgeExecutor {
            app: self.app.as_weak(),
            public_pem_path: self.public_pem_path.clone(),
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.app.run()
    }

    pub fn add_on_reset_commands(&self) {
        let (app_weak, cmds_list_mutex) = self.app_and_cmds();

        self.app.global::<SlintRustBridge>().on_reset_commands(move || {
            info("Resetting commands");

            Self::with_file_commands_list(&cmds_list_mutex, |cl| {
                let app = app_weak.unwrap();
                let command_logic = app.global::<SlintRustBridge>();
                command_logic.set_commands_config(cl.to_string().into());
            });
        });
    }

    pub fn add_on_set_commands_config(&self) {
        let (app_weak, cmds_list_mutex) = self.app_and_cmds();

        self.app.global::<SlintRustBridge>().on_set_commands_config(move |cmds| {
            info(&format!("Setting commands:\n{cmds}"));

            let cmds: Vec<String> = cmds.to_string().lines().map(str::to_string).collect();

            Self::with_app_commands_list(&app_weak, |cl| {
                let cmds: Vec<CommandData> =
                    cmds.iter().map(|c| Self::create_command_tuple(c)).collect();
                cl.set_vec(cmds);
            });

            Self::with_file_commands_list(&cmds_list_mutex, |cl| {
                cl.set(cmds);
            });
        });
    }

    pub fn add_on_del_command(&self) {
        let (app_weak, cmds_list_mutex) = self.app_and_cmds();

        self.app.global::<SlintRustBridge>().on_del_command(move |cmd, index| {
            info(&format!("Removing command: {cmd}"));

            Self::with_file_commands_list(&cmds_list_mutex, |cl| {
                cl.remove(cmd.to_string());
            });

            Self::with_app_commands_list(&app_weak, |cl| {
                cl.remove(index as usize);
            });
        });
    }

    pub fn add_on_exec_command(&self) {
        let app_weak = self.app.as_weak();
        let private_pem_path = self.private_pem_path.clone();

        self.app.global::<SlintRustBridge>().on_exec_command(move |cmd, idx| {
            let cmd = cmd.to_string();
            let mut cmd_vec: Vec<&str> = cmd.split_whitespace().collect();
            cmd_vec.insert(0, "ruroco");
            if !cmd.contains("--private-pem-path") {
                cmd_vec.push("--private-pem-path");
                cmd_vec.push(&private_pem_path);
            }

            info(&format!("Executing command: {}", cmd_vec.join(" ")));

            Self::with_app_commands_list(&app_weak, |cl| {
                match CliClient::try_parse_from(cmd_vec) {
                    Ok(cli_client) => run_client(cli_client)
                        .map(|_| Self::set_command_data_color(idx, cl, GREEN))
                        .unwrap_or_else(|_| Self::set_command_data_color(idx, cl, RED)),
                    Err(_) => Self::set_command_data_color(idx, cl, RED),
                };
            });
        })
    }

    pub fn add_on_add_command(&self) {
        let (app_weak, cmds_list_mutex) = self.app_and_cmds();

        self.app.global::<SlintRustBridge>().on_add_command(move |cmd| {
            info(&format!("Adding new command: {cmd}"));

            Self::with_file_commands_list(&cmds_list_mutex, |cl| {
                cl.add(cmd.to_string());
            });

            Self::with_app_commands_list(&app_weak, |cl| {
                cl.push(Self::create_command_tuple(cmd.as_ref()));
            });
        });
    }

    pub fn add_on_update_application(&self) {
        self.app.global::<SlintRustBridge>().on_update_application(move || {
            #[cfg(target_os = "linux")]
            {
                match Updater::create(false, None, None, false).unwrap().update() {
                    Ok(_) => {}
                    Err(err) => {
                        error(&format!("Error when updating application: {err}"));
                    }
                }
            }

            #[cfg(target_os = "android")]
            {
                Self::update_android();
            }
        });
    }

    fn with_file_commands_list<F>(cmds_list_mutex: &Arc<Mutex<CommandsList>>, f: F)
    where
        F: FnOnce(&mut MutexGuard<CommandsList>),
    {
        let mut commands_list = match cmds_list_mutex.lock() {
            Ok(m) => m,
            Err(e) => return error(&format!("Failed to acquire mutex lock: {e}")),
        };

        f(&mut commands_list);
    }

    fn with_app_commands_list<F>(app_weak: &Weak<App>, f: F)
    where
        F: FnOnce(&VecModel<CommandData>),
    {
        let commands_list_rc: ModelRc<CommandData> =
            app_weak.upgrade().unwrap().global::<SlintRustBridge>().get_commands_list();
        let commands_list: &VecModel<CommandData> = commands_list_rc
            .as_any()
            .downcast_ref()
            .expect("Expected an initialized commands_list, found None");
        f(commands_list);
    }

    fn app_and_cmds(&self) -> (Weak<App>, Arc<Mutex<CommandsList>>) {
        (self.app.as_weak(), Arc::clone(&self.commands_list))
    }

    fn get_commands_list_data(commands_list: &CommandsList) -> ModelRc<CommandData> {
        let commands_list_data: Vec<CommandData> =
            commands_list.get().iter().map(|cmd| Self::create_command_tuple(cmd)).collect();
        ModelRc::from(Rc::new(VecModel::from(commands_list_data)))
    }

    #[cfg(target_os = "android")]
    fn update_android() {
        let data = Updater::get_github_api_data(None).unwrap();
        let asset = data.assets.into_iter().find(|a| a.name.ends_with(".apk")).unwrap();

        let util = AndroidUtil::create();
        let uri = util.uri_parse(asset.browser_download_url).unwrap();
        let intent = util.new_view_intent(&uri).unwrap();
        let result = util.start_activity(&intent);
        let _ = result.inspect_err(|err| {
            info(&format!("Error (prob. expected) when opening browser window: {err}"))
        });
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

    fn create_command_tuple(command: &str) -> CommandData {
        CommandData {
            command: SharedString::from(command),
            name: SharedString::from(CommandsList::command_to_name(command)),
            color: Color::from_rgb_u8(204, 204, 204),
        }
    }
}
