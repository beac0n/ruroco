// Prevent console window in addition to Slint window in Windows release builds when,
// e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::{Model, ModelRc, SharedString, VecModel};
use std::error::Error;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.set_commands_list(ModelRc::from(Rc::new(VecModel::from(vec![]))));

    ui.on_add_command({
        let ui_handle = ui.as_weak();
        move |cmd| {
            let commands_list_rc = ui_handle.unwrap().get_commands_list();
            let commands_list: &VecModel<SharedString> = commands_list_rc
                .as_any()
                .downcast_ref()
                .expect("Expected an initialized commands_list, found None");

            commands_list.push(cmd.into());
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
        }
    });

    ui.on_exec_command(|cmd| {
        println!("{cmd}");
    });

    ui.run()?;

    Ok(())
}
