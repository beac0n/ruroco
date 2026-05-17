#[cfg(target_os = "android")]
use crate::common::logging::error;
use crate::ui::app::{PasteTarget, RurocoApp};
use crate::ui::command_data::command_to_data;
use eframe::egui;

pub(crate) fn render(app: &mut RurocoApp, ui: &mut egui::Ui, config_height: f32) {
    ui.add_space(10.0);

    egui::ScrollArea::vertical().max_height(config_height).id_salt("config_scroll").show(
        ui,
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.commands_config_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        },
    );

    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let btn_w = (ui.available_width() - 24.0) / 4.0;
        if ui.add_sized([btn_w, 50.0], egui::Button::new("Reset")).clicked() {
            app.commands_config_text = app.commands_list.to_string();
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("💾")).clicked() {
            let cmds: Vec<_> = app.commands_config_text.lines().map(command_to_data).collect();
            app.commands_list.set(cmds);
            app.sync_config_text();
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("📋")).clicked() {
            ui.ctx().copy_text(app.commands_config_text.clone());
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("📥")).clicked() {
            #[cfg(target_os = "android")]
            match crate::common::android::AndroidClipboard::get_text() {
                Ok(text) => app.commands_config_text = text,
                Err(e) => error(format!("Failed to paste: {e}")),
            }
            #[cfg(not(target_os = "android"))]
            {
                app.paste_target = Some(PasteTarget::Config);
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
        }
    });
}
