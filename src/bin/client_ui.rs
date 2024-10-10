// Prevent console window in addition to Slint window in Windows release builds when,
// e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use ruroco::client::exec_cli_client;
use ruroco::common::{error, NTP_SYSTEM};
use ruroco::config_client::{
    default_private_pem_path, default_public_pem_path, CliClient, DEFAULT_COMMAND,
    DEFAULT_DEADLINE, DEFAULT_KEY_SIZE, MIN_KEY_SIZE,
};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::error::Error;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    let private_pem_path = default_private_pem_path().to_str().unwrap_or("").to_string();
    let public_pem_path = default_public_pem_path().to_str().unwrap_or("").to_string();

    // TODO: load commands list from disk
    // TODO: figure out which path to use on android
    ui.set_commands_list(ModelRc::from(Rc::new(VecModel::from(vec![]))));
    ui.set_private_pem_path(SharedString::from(private_pem_path));
    ui.set_public_pem_path(SharedString::from(public_pem_path));
    ui.set_key_size(DEFAULT_KEY_SIZE.into());
    ui.set_min_key_size(MIN_KEY_SIZE.into());
    ui.set_command(SharedString::from(DEFAULT_COMMAND));
    ui.set_deadline(DEFAULT_DEADLINE.into());
    ui.set_ntp(SharedString::from(NTP_SYSTEM));

    ui.on_add_command({
        let ui_handle = ui.as_weak();
        move |cmd| {
            let commands_list_rc = ui_handle.unwrap().get_commands_list();
            let commands_list: &VecModel<SharedString> = commands_list_rc
                .as_any()
                .downcast_ref()
                .expect("Expected an initialized commands_list, found None");

            commands_list.push(cmd);
            // TODO: save cmd to disk in config file
        }
    });

    ui.on_del_command({
        let ui_handle = ui.as_weak();
        move |cmd| {
            let commands_list_rc = ui_handle.unwrap().get_commands_list();
            let commands_list: &VecModel<SharedString> = commands_list_rc
                .as_any()
                .downcast_ref()
                .expect("Expected an initialized commands_list, found None");

            commands_list
                .iter()
                .enumerate()
                .find_map(|(idx, entry)| if entry == cmd { Some(idx) } else { None })
                .map(|idx| commands_list.remove(idx));

            // TODO: remove cmd on disk in config file
        }
    });

    ui.on_exec_command(|cmd| {
        let cmd_str = cmd.to_string();
        let mut cmd_vec: Vec<&str> = cmd_str.split(" ").collect();
        cmd_vec.insert(0, "ruroco");
        match CliClient::try_parse_from(cmd_vec) {
            Ok(cli_client) => exec_cli_client(cli_client)
                .unwrap_or_else(|e| error(&format!("Failed to execute \"{cmd_str}\": {e}"))),
            Err(e) => error(&format!("Failed to create cli client from \"{cmd_str}\": {e}")),
        };
    });

    ui.run()?;

    Ok(())
}
