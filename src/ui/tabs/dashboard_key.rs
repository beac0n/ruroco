use crate::common::crypto_handler::CryptoHandler;
use crate::common::logging::error;
use crate::ui::app::{PasteTarget, RurocoApp};
use crate::ui::tabs::widgets;
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

    let lock_label = if app.show_key { "🔒" } else { "🔓" };
    let r = widgets::equal_buttons(ui, &["Generate", lock_label, "📋", "📥"]);
    if r[0] {
        match CryptoHandler::gen_key() {
            Ok(k) => app.key = k,
            Err(e) => error(format!("Failed to generate key: {e}")),
        }
    }
    if r[1] {
        app.show_key = !app.show_key;
    }
    if r[2] {
        widgets::copy_text(ui, &app.key);
    }
    if r[3] {
        widgets::paste_button(app, ui, PasteTarget::Key);
    }
}
