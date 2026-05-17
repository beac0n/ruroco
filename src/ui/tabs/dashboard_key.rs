use crate::common::crypto_handler::CryptoHandler;
use crate::common::logging::error;
use crate::ui::app::{PasteTarget, RurocoApp};
use eframe::egui;

pub(crate) fn render(app: &mut RurocoApp, ui: &mut egui::Ui) {
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
            match crate::common::android_clipboard::get_clipboard_text() {
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
}
