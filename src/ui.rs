use crate::client::{gen, run_client};
use crate::common::{error, info, NTP_SYSTEM};
use crate::config_client::{CliClient, DEFAULT_COMMAND, DEFAULT_DEADLINE, DEFAULT_KEY_SIZE};
use crate::saved_command_list::CommandsList;
use clap::Parser;

use slint::{Model, ModelRc, SharedString, VecModel};
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::{env, fs};

slint::include_modules!();
pub fn run_ui(private_files_path: String) -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    let private_pem_path_str = format!("{private_files_path}/ruroco_private.pem").to_string();
    let public_pem_path_str = format!("{private_files_path}/ruroco_public.pem").to_string();

    let public_pem_path = PathBuf::from(public_pem_path_str);
    match (PathBuf::from(&private_pem_path_str), public_pem_path.clone()) {
        (pr, pu) if !pr.exists() && !pu.exists() => {
            gen(pr, pu, DEFAULT_KEY_SIZE as u32)?;
        }
        (pr, pu) if pr.exists() && pu.exists() => {}
        (pr, pu) if pr.exists() && !pu.exists() => {
            fs::remove_file(&pr)?;
            gen(pr, pu, DEFAULT_KEY_SIZE as u32)?;
        }
        (pr, pu) => {
            fs::remove_file(&pu)?;
            gen(pr, pu, DEFAULT_KEY_SIZE as u32)?;
        }
    }

    let commands_list = CommandsList::create(&PathBuf::from(&private_files_path));
    ui.set_commands_list(ModelRc::from(Rc::new(VecModel::from(commands_list.get()))));
    ui.set_private_pem_path(SharedString::from(private_pem_path_str));
    ui.set_public_key(fs::read_to_string(&public_pem_path)?.into());

    ui.set_command(SharedString::from(DEFAULT_COMMAND));
    ui.set_deadline(DEFAULT_DEADLINE.to_string().into());
    ui.set_ntp(SharedString::from(NTP_SYSTEM));

    ui.on_add_command({
        let ui_handle = ui.as_weak();
        let mut persistent_commands_list =
            CommandsList::create(&PathBuf::from(&private_files_path));
        move |cmd| {
            let commands_list_rc = ui_handle.unwrap().get_commands_list();
            let commands_list: &VecModel<SharedString> = commands_list_rc
                .as_any()
                .downcast_ref()
                .expect("Expected an initialized commands_list, found None");

            info(&format!("Adding new command: {cmd}"));
            persistent_commands_list.add(cmd.clone());
            commands_list.push(cmd);
        }
    });

    ui.on_del_command({
        let ui_handle = ui.as_weak();
        let mut persistent_commands_list =
            CommandsList::create(&PathBuf::from(&private_files_path));
        move |cmd| {
            let commands_list_rc = ui_handle.unwrap().get_commands_list();
            let commands_list: &VecModel<SharedString> = commands_list_rc
                .as_any()
                .downcast_ref()
                .expect("Expected an initialized commands_list, found None");

            info(&format!("Removing command: {cmd}"));
            persistent_commands_list.remove(cmd.clone());
            commands_list
                .iter()
                .enumerate()
                .find_map(|(idx, entry)| if entry == cmd { Some(idx) } else { None })
                .map(|idx| commands_list.remove(idx));
        }
    });

    ui.on_exec_command(|cmd| {
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

    ui.run()?;

    Ok(())
}
