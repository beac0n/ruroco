use crate::client::config::{get_conf_dir, SendCommand};
use crate::client::send::Sender;
use crate::common::logging::{error, info};
use crate::ui::app::{ExecuteState, Status, StatusKey};
use crate::ui::colors;
use crate::ui::command_data::CommandData;
use crate::ui::saved_command_list::CommandsList;
use crate::ui::tabs::widgets;
use anyhow::Context;
use eframe::egui;
use std::io::Write;
use tempfile::NamedTempFile;

pub(crate) fn render(
    state: &mut ExecuteState,
    commands_list: &mut CommandsList,
    key: &str,
    ui: &mut egui::Ui,
) {
    let cmds = commands_list.get().to_vec();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut to_delete: Option<CommandData> = None;
        let mut to_exec: Option<CommandData> = None;

        for cmd in &cmds {
            let status_color = state.color_for(cmd);
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if widgets::Widgets::new(ui).icon_button(colors::BLUE, "▶").clicked() {
                    to_exec = Some(cmd.clone());
                }

                // Right-to-left: delete anchored right, name fills middle
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::Widgets::new(ui).icon_button(colors::RED, "🗑").clicked() {
                        to_delete = Some(cmd.clone());
                    }

                    // inner_margin(4.0) adds 8px; buttons outer = 48px → inner = 40px
                    widgets::Widgets::bordered(status_color, 4.0).show(ui, |ui| {
                        ui.set_min_height(40.0);
                        ui.set_max_height(40.0);
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.add(egui::Label::new(&cmd.name).wrap());
                            },
                        );
                    });
                });
            });

            ui.add_space(4.0);
        }

        if let Some(cmd) = to_delete {
            state.status.remove(&StatusKey::from(&cmd));
            commands_list.remove(&cmd);
        }

        if let Some(cmd) = to_exec {
            exec_command(state, key, cmd);
        }
    });
}

fn exec_command(state: &mut ExecuteState, key: &str, cmd: CommandData) {
    info(format!("Executing command: {}", cmd.name));

    let result = write_key_file(key).and_then(|key_file| {
        let ip = cmd.ip.trim();
        let send_command = SendCommand {
            address: cmd.address.clone(),
            command: cmd.command.clone(),
            permissive: cmd.permissive,
            ip: if ip.is_empty() {
                None
            } else {
                Some(ip.to_string())
            },
            ipv4: cmd.ipv4,
            ipv6: cmd.ipv6,
            key_file: key_file.path().to_path_buf(),
            ..Default::default()
        };

        // key_file (a NamedTempFile) is kept alive until Sender::create has read it, then
        // dropped, which removes it from disk. The key never lives in a SendCommand field.
        Sender::create(send_command).and_then(|mut sender| sender.send())
    });

    match result {
        Ok(_) => {
            state.set(&cmd, Status::Ok);
        }
        Err(e) => {
            error(format!("Error executing command '{}': {e}", cmd.name));
            state.set(&cmd, Status::Err);
        }
    }
}

/// Writes `key` to a temporary file (auto-deleted on drop) so it can be passed to `Sender::create`
/// via `key_file`. The GUI holds the key in memory; `SendCommand` only ever accepts one via a file.
///
/// Created inside the conf dir rather than the platform temp dir: on Android there is no writable
/// `/tmp` (the sandboxed app has no `$TMPDIR`, so `NamedTempFile::new()` would resolve to an
/// unwritable `/tmp` and fail every send), whereas `get_conf_dir()` already resolves to a writable,
/// per-platform location (JNI `getFilesDir()` on Android, `$HOME/.config/ruroco` elsewhere).
fn write_key_file(key: &str) -> anyhow::Result<NamedTempFile> {
    let conf_dir = get_conf_dir().with_context(|| "Could not determine conf dir")?;
    let mut file = NamedTempFile::new_in(&conf_dir)
        .with_context(|| format!("Could not create temporary key file in {conf_dir:?}"))?;
    file.write_all(key.trim().as_bytes()).with_context(|| "Could not write temporary key file")?;
    Ok(file)
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use crate::ui::app::ExecuteState;
    use crate::ui::colors;
    use crate::ui::saved_command_list::CommandsList;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use std::collections::HashMap;

    fn make_cmd() -> CommandData {
        CommandData {
            address: "127.0.0.1:1234".into(),
            command: "default".into(),
            permissive: false,
            ip: String::new(),
            ipv4: false,
            ipv6: false,
            name: "default@127.0.0.1:1234".into(),
        }
    }

    #[test]
    fn test_render_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = ExecuteState {
            status: HashMap::new(),
        };
        let mut commands_list = CommandsList::create(dir.path());
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut state, &mut commands_list, "", ui);
        });
        harness.run();
    }

    #[test]
    fn test_render_with_commands_covers_loop_body() {
        let dir = tempfile::tempdir().unwrap();
        let mut commands_list = CommandsList::create(dir.path());
        commands_list.add(make_cmd());
        let mut state = ExecuteState {
            status: HashMap::new(),
        };
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut state, &mut commands_list, "", ui);
        });
        harness.run();
    }

    #[test]
    fn test_render_delete_command() {
        let dir = tempfile::tempdir().unwrap();
        let mut commands_list = CommandsList::create(dir.path());
        commands_list.add(make_cmd());
        let state: (ExecuteState, CommandsList) = (
            ExecuteState {
                status: HashMap::new(),
            },
            commands_list,
        );
        let mut harness = Harness::new_ui_state(
            |ui, (st, cl): &mut (ExecuteState, CommandsList)| {
                render(st, cl, "", ui);
            },
            state,
        );
        harness.get_by_label("🗑").click();
        harness.run();
        assert!(harness.state().1.get().is_empty());
    }

    #[test]
    fn test_play_button_triggers_exec() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        let mut commands_list = CommandsList::create(dir.path());
        commands_list.add(make_cmd());
        let state: (ExecuteState, CommandsList) = (
            ExecuteState {
                status: HashMap::new(),
            },
            commands_list,
        );
        let mut harness = Harness::new_ui_state(
            |ui, (st, cl): &mut (ExecuteState, CommandsList)| {
                render(st, cl, "", ui);
            },
            state,
        );
        harness.get_by_label("▶").click();
        harness.run();
        let cmd = make_cmd();
        assert_eq!(harness.state().0.color_for(&cmd), colors::RED);
    }

    #[test]
    fn test_exec_command_empty_key_sets_err() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        let mut state = ExecuteState {
            status: HashMap::new(),
        };
        let cmd = make_cmd();
        super::exec_command(&mut state, "", cmd.clone());
        assert_eq!(state.color_for(&cmd), colors::RED);
    }

    #[test]
    fn test_exec_command_with_valid_key_sets_ok() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        let key = crate::client::gen::Generator::create().unwrap().gen().unwrap();
        let mut state = ExecuteState {
            status: HashMap::new(),
        };
        let cmd = make_cmd();
        super::exec_command(&mut state, &key, cmd.clone());
        assert_eq!(state.color_for(&cmd), colors::GREEN);
    }
}
