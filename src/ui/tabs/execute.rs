use crate::client::config::CliClient;
use crate::client::run_client_send;
use crate::common::logging::error;
use crate::ui::app::{RurocoApp, Status, StatusKey};
use crate::ui::command_data::{data_to_command, CommandData};
use clap::Parser;
use eframe::egui;

pub(crate) fn render(app: &mut RurocoApp, ui: &mut egui::Ui) {
    let cmds: Vec<CommandData> = app.cached_commands.clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut to_delete: Option<CommandData> = None;
        let mut to_exec: Option<CommandData> = None;

        for cmd in &cmds {
            let status_color = app.status_color(cmd);
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                // Play button — fixed left
                let play_frame = egui::Frame::default()
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(25, 118, 210)))
                    .corner_radius(5.0)
                    .inner_margin(1.0);
                play_frame.show(ui, |ui| {
                    if ui.add_sized([46.0, 46.0], egui::Button::new("▶")).clicked() {
                        to_exec = Some(cmd.clone());
                    }
                });

                // Right-to-left sub-layout: delete first (anchored right), then name fills middle
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let del_frame = egui::Frame::default()
                        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(211, 47, 47)))
                        .corner_radius(5.0)
                        .inner_margin(1.0);
                    del_frame.show(ui, |ui| {
                        if ui.add_sized([46.0, 46.0], egui::Button::new("🗑")).clicked() {
                            to_delete = Some(cmd.clone());
                        }
                    });

                    let name_frame = egui::Frame::default()
                        .stroke(egui::Stroke::new(2.0, status_color))
                        .corner_radius(5.0)
                        .inner_margin(4.0);
                    name_frame.show(ui, |ui| {
                        // inner_margin(4.0) adds 8px; buttons outer = 48px → inner = 40px
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
            app.command_status.remove(&StatusKey::from(&cmd));
            app.commands_list.remove(&cmd);
            app.refresh_cache();
        }

        if let Some(cmd) = to_exec {
            exec_command(app, cmd);
        }
    });
}

fn exec_command(app: &mut RurocoApp, cmd: CommandData) {
    use crate::common::logging::info;
    info(format!("Executing command: {}", cmd.name));

    let key = app.key.trim().to_string();
    let cmd_str = data_to_command(&cmd, if key.is_empty() { None } else { Some(key) });
    let mut cmd_vec: Vec<&str> = cmd_str.split_whitespace().collect();
    cmd_vec.insert(0, "ruroco");

    let result =
        CliClient::try_parse_from(cmd_vec).map_err(anyhow::Error::from).and_then(run_client_send);

    match result {
        Ok(_) => {
            app.set_status(&cmd, Status::Ok);
        }
        Err(e) => {
            error(format!("Error executing command '{}': {e}", cmd.name));
            app.set_status(&cmd, Status::Err);
        }
    }
}
