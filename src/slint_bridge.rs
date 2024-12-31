use crate::client::run_client;
use crate::common::{error, info, NTP_SYSTEM};
use crate::config_client::{get_conf_dir, CliClient, DEFAULT_COMMAND, DEFAULT_DEADLINE};
use crate::saved_command_list::CommandsList;
use clap::Parser;
use slint::{Color, Model, ModelRc, SharedString, VecModel, Weak};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

pub struct SlintBridge {
    // app: App,
    //command_logic: CommandLogic<'a>,
    cmds_list_arc_mutex: Arc<Mutex<CommandsList>>,
}

impl SlintBridge {
    pub fn create(
        app: &App,
        private_pem_path: PathBuf,
        public_pem_path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let commands_list = CommandsList::create(&get_conf_dir());
        let commands_list_data = commands_list.get();
        let cmds_list_arc_mutex = Arc::new(Mutex::new(commands_list));

        let command_logic = app.global::<CommandLogic>();

        command_logic.set_commands_list(ModelRc::from(Rc::new(VecModel::from(commands_list_data))));
        command_logic.set_private_pem_path(SharedString::from(
            private_pem_path.to_str().ok_or("Could not convert path to string")?,
        ));
        command_logic.set_public_key(fs::read_to_string(&public_pem_path)?.into());
        command_logic.set_command(SharedString::from(DEFAULT_COMMAND));
        command_logic.set_deadline(DEFAULT_DEADLINE.to_string().into());
        command_logic.set_ntp(SharedString::from(NTP_SYSTEM));

        Ok(SlintBridge {
            //app,
            //command_logic,
            cmds_list_arc_mutex,
        })
    }

    pub fn add_on_del_command(&self, app: &App) {
        let app_weak = app.as_weak();

        app.global::<CommandLogic>().on_del_command({
            let cmds_list_mutex = Arc::clone(&self.cmds_list_arc_mutex);
            move |cmd, index| {
                info(&format!("Removing command: {cmd}"));
                let mut persistent_commands_list = match cmds_list_mutex.lock() {
                    Ok(m) => m,
                    Err(e) => return error(&format!("Failed to acquire mutex lock: {e}")),
                };
                persistent_commands_list.remove(cmd);

                let commands_list_rc = Self::get_commands_list_rc(&app_weak);
                let commands_list = Self::get_commands_list(&commands_list_rc);
                commands_list.remove(index as usize);
            }
        });
    }

    pub fn add_on_exec_command(&self, app: &App) {
        let app_weak = app.as_weak();

        app.global::<CommandLogic>().on_exec_command({
            move |cmd, idx| {
                let commands_list_rc = Self::get_commands_list_rc(&app_weak);
                let commands_list = Self::get_commands_list(&commands_list_rc);

                let cmd_str = cmd.to_string();
                let mut cmd_vec: Vec<&str> = cmd_str.split_whitespace().collect();
                cmd_vec.insert(0, "ruroco");

                info(&format!("Executing command: {cmd}"));
                match CliClient::try_parse_from(cmd_vec) {
                    Ok(cli_client) => run_client(cli_client)
                        .map(|_| {
                            Self::set_command_data_color(
                                idx,
                                commands_list,
                                Color::from_rgb_u8(56, 142, 60),
                            )
                        })
                        .unwrap_or_else(|_| {
                            Self::set_command_data_color(
                                idx,
                                commands_list,
                                Color::from_rgb_u8(211, 47, 47),
                            )
                        }),
                    Err(_) => Self::set_command_data_color(
                        idx,
                        commands_list,
                        Color::from_rgb_u8(211, 47, 47),
                    ),
                };
            }
        })
    }

    pub fn add_on_add_command(&self, app: &App) {
        let app_weak = app.as_weak();

        app.global::<CommandLogic>().on_add_command({
            let cmds_list_mutex = Arc::clone(&self.cmds_list_arc_mutex);
            move |cmd| {
                info(&format!("Adding new command: {cmd}"));
                let mut persistent_commands_list = match cmds_list_mutex.lock() {
                    Ok(m) => m,
                    Err(e) => return error(&format!("Failed to acquire mutex lock: {e}")),
                };
                persistent_commands_list.add(cmd.clone());

                let commands_list_rc = Self::get_commands_list_rc(&app_weak);
                let commands_list = Self::get_commands_list(&commands_list_rc);
                commands_list.push(CommandsList::create_command_tuple(cmd));
            }
        });
    }

    fn get_commands_list_rc(app_handle: &Weak<App>) -> ModelRc<CommandData> {
        app_handle.unwrap().global::<CommandLogic>().get_commands_list()
    }

    fn get_commands_list(commands_list_rc: &ModelRc<CommandData>) -> &VecModel<CommandData> {
        commands_list_rc
            .as_any()
            .downcast_ref()
            .expect("Expected an initialized commands_list, found None")
    }

    fn set_command_data_color(idx: i32, commands_list: &VecModel<CommandData>, color: Color) {
        let command_data_vec: Vec<CommandData> = commands_list
            .iter()
            .enumerate()
            .map(|(i, d)| CommandData {
                command: d.command,
                name: d.name,
                color: if i == idx as usize {
                    color
                } else {
                    Color::from_rgb_u8(204, 204, 204)
                },
            })
            .collect();

        commands_list.set_vec(command_data_vec);
    }
}
