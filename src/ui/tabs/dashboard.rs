#[cfg(target_os = "linux")]
use crate::client::update::Updater;
use crate::common::logging::error;
#[cfg(target_os = "android")]
use crate::ui::android_update::update_android;
use crate::ui::app::{PasteTarget, RurocoApp};
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

    let config_height = ui.available_height() * 0.45;
    super::dashboard_config::render(app, ui, config_height);

    ui.separator();
    ui.add_space(6.0);

    super::dashboard_key::render(app, ui);

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
