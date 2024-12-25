use crate::client::{gen, run_client};
use crate::common::{error, info, NTP_SYSTEM};
use crate::config_client::{
    default_private_pem_path, default_public_pem_path, get_conf_dir, CliClient, DEFAULT_COMMAND,
    DEFAULT_DEADLINE, DEFAULT_KEY_SIZE,
};
use crate::saved_command_list::CommandsList;
use crate::slint_bridge;
use crate::slint_bridge::CommandTuple;
use clap::Parser;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use slint_bridge::{App, CommandLogic};
use std::error::Error;
use std::fs;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let app = App::new()?;

    let public_pem_path = default_public_pem_path();
    let private_pem_path = default_private_pem_path();
    match (private_pem_path.clone(), public_pem_path.clone()) {
        (pr, pu) if !pr.exists() && !pu.exists() => {
            gen(pr, pu, DEFAULT_KEY_SIZE as u32)?;
        }
        (pr, pu) if pr.exists() && pu.exists() => {}
        (_, _) => {
            Err("Invalid public/privat pem state - there should be both or neither".to_string())?
        }
    }

    let commands_list = CommandsList::create(&get_conf_dir());

    let globals = app.global::<CommandLogic>();
    globals.set_commands_list(ModelRc::from(Rc::new(VecModel::from(commands_list.get()))));
    globals.set_private_pem_path(SharedString::from(
        private_pem_path.to_str().ok_or("Could not convert path to string")?,
    ));
    globals.set_public_key(fs::read_to_string(&public_pem_path)?.into());
    globals.set_command(SharedString::from(DEFAULT_COMMAND));
    globals.set_deadline(DEFAULT_DEADLINE.to_string().into());
    globals.set_ntp(SharedString::from(NTP_SYSTEM));

    let cmds_list_arc_mutex = Arc::new(Mutex::new(CommandsList::create(&get_conf_dir())));

    globals.on_add_command({
        let app_handle = app.as_weak();
        let cmds_list_mutex = Arc::clone(&cmds_list_arc_mutex);
        move |cmd| {
            info(&format!("Adding new command: {cmd}"));
            let mut persistent_commands_list = cmds_list_mutex.lock().unwrap();
            persistent_commands_list.add(cmd.clone());

            let commands_list_rc = get_commands_list_rc(&app_handle);
            let commands_list = get_commands_list(&commands_list_rc);
            commands_list.push(CommandsList::create_command_tuple(cmd));
        }
    });

    globals.on_del_command({
        let app_handle = app.as_weak();
        let cmds_list_mutex = Arc::clone(&cmds_list_arc_mutex);

        move |cmd| {
            info(&format!("Removing command: {cmd}"));
            let mut persistent_commands_list = cmds_list_mutex.lock().unwrap();
            persistent_commands_list.remove(cmd.clone());

            let commands_list_rc = get_commands_list_rc(&app_handle);
            let commands_list = get_commands_list(&commands_list_rc);
            commands_list
                .iter()
                .enumerate()
                .find_map(|(idx, entry)| {
                    if entry.command == cmd {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .map(|idx| commands_list.remove(idx));
        }
    });

    globals.on_exec_command(|cmd| {
        let cmd_str = cmd.to_string();
        let mut cmd_vec: Vec<&str> = cmd_str.split(" ").collect();
        cmd_vec.insert(0, "ruroco");

        info(&format!("Executing command: {cmd}"));
        match CliClient::try_parse_from(cmd_vec) {
            Ok(cli_client) => run_client(cli_client)
                .unwrap_or_else(|e| error(&format!("Failed to execute \"{cmd_str}\": {e}"))),
            Err(e) => error(&format!("Failed to create cli client from \"{cmd_str}\": {e}")),
        };
    });

    app.run()?;

    Ok(())
}

fn get_commands_list_rc(app_handle: &Weak<App>) -> ModelRc<CommandTuple> {
    app_handle.unwrap().global::<CommandLogic>().get_commands_list()
}

fn get_commands_list(commands_list_rc: &ModelRc<CommandTuple>) -> &VecModel<CommandTuple> {
    commands_list_rc
        .as_any()
        .downcast_ref()
        .expect("Expected an initialized commands_list, found None")
}
