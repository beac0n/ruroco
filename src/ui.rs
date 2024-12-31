use crate::client::{gen, run_client};
use crate::common::{error, info, NTP_SYSTEM};
use crate::config_client::{
    default_private_pem_path, default_public_pem_path, get_conf_dir, CliClient, DEFAULT_COMMAND,
    DEFAULT_DEADLINE, DEFAULT_KEY_SIZE,
};
use crate::saved_command_list::CommandsList;
use crate::slint_bridge;
use crate::slint_bridge::CommandData;
use clap::Parser;
use slint::{Color, ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use slint_bridge::{App, CommandLogic};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let app = App::new()?;

    let public_pem_path = default_public_pem_path();
    let private_pem_path = default_private_pem_path();
    generate_pem_files(&public_pem_path, &private_pem_path)?;

    let commands_list = CommandsList::create(&get_conf_dir());
    let commands_list_data = commands_list.get();
    let cmds_list_arc_mutex = Arc::new(Mutex::new(commands_list));

    let globals = app.global::<CommandLogic>();
    globals.set_commands_list(ModelRc::from(Rc::new(VecModel::from(commands_list_data))));
    globals.set_private_pem_path(SharedString::from(
        private_pem_path.to_str().ok_or("Could not convert path to string")?,
    ));
    globals.set_public_key(fs::read_to_string(&public_pem_path)?.into());
    globals.set_command(SharedString::from(DEFAULT_COMMAND));
    globals.set_deadline(DEFAULT_DEADLINE.to_string().into());
    globals.set_ntp(SharedString::from(NTP_SYSTEM));

    globals.on_add_command({
        let app_handle = app.as_weak();
        let cmds_list_mutex = Arc::clone(&cmds_list_arc_mutex);
        move |cmd| {
            info(&format!("Adding new command: {cmd}"));
            let mut persistent_commands_list = match cmds_list_mutex.lock() {
                Ok(m) => m,
                Err(e) => return error(&format!("Failed to acquire mutex lock: {e}")),
            };
            persistent_commands_list.add(cmd.clone());

            let commands_list_rc = get_commands_list_rc(&app_handle);
            let commands_list = get_commands_list(&commands_list_rc);
            commands_list.push(CommandsList::create_command_tuple(cmd));
        }
    });

    globals.on_del_command({
        let app_handle = app.as_weak();
        let cmds_list_mutex = Arc::clone(&cmds_list_arc_mutex);
        move |cmd, index| {
            info(&format!("Removing command: {cmd}"));
            let mut persistent_commands_list = match cmds_list_mutex.lock() {
                Ok(m) => m,
                Err(e) => return error(&format!("Failed to acquire mutex lock: {e}")),
            };
            persistent_commands_list.remove(cmd);

            let commands_list_rc = get_commands_list_rc(&app_handle);
            let commands_list = get_commands_list(&commands_list_rc);
            commands_list.remove(index as usize);
        }
    });

    globals.on_exec_command({
        let app_handle = app.as_weak();

        move |cmd, idx| {
            let commands_list_rc = get_commands_list_rc(&app_handle);
            let commands_list = get_commands_list(&commands_list_rc);

            let cmd_str = cmd.to_string();
            let mut cmd_vec: Vec<&str> = cmd_str.split_whitespace().collect();
            cmd_vec.insert(0, "ruroco");

            info(&format!("Executing command: {cmd}"));
            match CliClient::try_parse_from(cmd_vec) {
                Ok(cli_client) => run_client(cli_client)
                    .map(|_| {
                        set_command_data_color(idx, commands_list, Color::from_rgb_u8(56, 142, 60))
                    })
                    .unwrap_or_else(|_| {
                        set_command_data_color(idx, commands_list, Color::from_rgb_u8(211, 47, 47))
                    }),
                Err(_) => {
                    set_command_data_color(idx, commands_list, Color::from_rgb_u8(211, 47, 47))
                }
            };
        }
    });

    app.run()?;

    Ok(())
}

fn generate_pem_files(
    public_pem_path: &PathBuf,
    private_pem_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    match (private_pem_path.clone(), public_pem_path.clone()) {
        (priv_pp, pub_pp) if !priv_pp.exists() && !pub_pp.exists() => {
            gen(priv_pp, pub_pp, DEFAULT_KEY_SIZE as u32)?;
        }
        (pr, pu) if pr.exists() && pu.exists() => {}
        (_, _) => {
            Err("Invalid public/privat pem state - there should be both or neither".to_string())?
        }
    }
    Ok(())
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

fn get_commands_list_rc(app_handle: &Weak<App>) -> ModelRc<CommandData> {
    app_handle.unwrap().global::<CommandLogic>().get_commands_list()
}

fn get_commands_list(commands_list_rc: &ModelRc<CommandData>) -> &VecModel<CommandData> {
    commands_list_rc
        .as_any()
        .downcast_ref()
        .expect("Expected an initialized commands_list, found None")
}
