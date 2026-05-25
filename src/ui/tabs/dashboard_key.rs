use crate::common::crypto_handler::CryptoHandler;
use crate::common::logging::error;
use crate::ui::app::{DashboardState, PasteTarget};
use crate::ui::tabs::widgets;
use eframe::egui;

pub(crate) fn render(dashboard: &mut DashboardState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("AES Key:");
        ui.add(
            egui::TextEdit::singleline(&mut dashboard.key)
                .hint_text("Generate or paste your key here")
                .password(!dashboard.show_key)
                .desired_width(f32::INFINITY),
        );
    });

    ui.add_space(6.0);

    let lock_label = if dashboard.show_key { "🔒" } else { "🔓" };
    let r = widgets::Widgets::new(ui).equal_buttons(&["Generate", lock_label, "📋", "📥"]);
    if r[0] {
        match CryptoHandler::gen_key() {
            Ok(k) => dashboard.save_key(k),
            Err(e) => error(format!("Failed to generate key: {e}")),
        }
    }
    if r[1] {
        dashboard.show_key = !dashboard.show_key;
    }
    if r[2] {
        widgets::Widgets::new(ui).copy_text(&dashboard.key);
    }
    if r[3] {
        widgets::Widgets::new(ui).paste_button(dashboard, PasteTarget::Key);
    }
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    #[test]
    fn test_render_runs() {
        let mut dashboard = DashboardState {
            config_text: String::new(),
            key: String::new(),
            show_key: false,
            paste_target: None,
        };
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut dashboard, ui);
        });
        harness.run();
    }
}
