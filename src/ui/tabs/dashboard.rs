#[cfg(target_os = "linux")]
use crate::client::update::Updater;
use crate::common::crypto_handler::CryptoHandler;
use crate::common::logging::error;
#[cfg(target_os = "android")]
use crate::ui::android_update::update_android;
use crate::ui::app::{PasteTarget, RurocoApp};
use crate::ui::command_data::command_to_data;
use eframe::egui;

pub(crate) fn render(app: &mut RurocoApp, ui: &mut egui::Ui) {
    if let Some(text) = ui.ctx().input(|i| {
        i.events.iter().find_map(|e| {
            if let egui::Event::Paste(t) = e {
                Some(t.clone())
            } else {
                None
            }
        })
    }) {
        if let Some(target) = app.paste_target.take() {
            match target {
                PasteTarget::Key => app.key = text,
                PasteTarget::Config => app.commands_config_text = text,
            }
        }
    }

    let available = ui.available_height();
    let config_height = available * 0.45;

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
            app.refresh_cache();
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("📋")).clicked() {
            ui.ctx().copy_text(app.commands_config_text.clone());
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("📥")).clicked() {
            #[cfg(target_os = "android")]
            match crate::common::android_util::get_clipboard_text() {
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

    ui.separator();
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("AES Key:");
        ui.add(
            egui::TextEdit::singleline(&mut app.key)
                .hint_text("Generate or paste your key here")
                .password(!app.show_key)
                .desired_width(f32::INFINITY),
        );
    });

    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let btn_w = (ui.available_width() - 24.0) / 4.0;
        if ui.add_sized([btn_w, 50.0], egui::Button::new("Generate")).clicked() {
            match CryptoHandler::gen_key() {
                Ok(k) => app.key = k,
                Err(e) => error(format!("Failed to generate key: {e}")),
            }
        }
        let lock_label = if app.show_key { "🔒" } else { "🔓" };
        if ui.add_sized([btn_w, 50.0], egui::Button::new(lock_label)).clicked() {
            app.show_key = !app.show_key;
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("📋")).clicked() {
            ui.ctx().copy_text(app.key.clone());
        }
        if ui.add_sized([btn_w, 50.0], egui::Button::new("📥")).clicked() {
            #[cfg(target_os = "android")]
            match crate::common::android_util::get_clipboard_text() {
                Ok(text) => app.key = text,
                Err(e) => error(format!("Failed to paste: {e}")),
            }
            #[cfg(not(target_os = "android"))]
            {
                app.paste_target = Some(PasteTarget::Key);
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
        }
    });

    ui.add_space(10.0);

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        if ui
            .add_sized([ui.available_width(), 50.0], egui::Button::new("Update Application"))
            .clicked()
        {
            #[cfg(target_os = "linux")]
            if let Err(e) = Updater::create(false, None, None, false).and_then(|u| u.update()) {
                error(format!("Update failed: {e}"));
            }
            #[cfg(target_os = "android")]
            if let Err(e) = update_android() {
                error(format!("Update failed: {e}"));
            }
        }
    });
}
