use crate::ui::app::{PasteTarget, RurocoApp};
use eframe::egui;

pub(crate) fn equal_buttons(ui: &mut egui::Ui, labels: &[&str]) -> Vec<bool> {
    let n = labels.len() as f32;
    let btn_w = (ui.available_width() - (n - 1.0) * 8.0) / n;
    let mut clicked = vec![false; labels.len()];
    ui.horizontal(|ui| {
        for (i, label) in labels.iter().enumerate() {
            clicked[i] = ui.add_sized([btn_w, 50.0], egui::Button::new(*label)).clicked();
        }
    });
    clicked
}

pub(crate) fn copy_text(ui: &egui::Ui, text: &str) {
    #[cfg(target_os = "android")]
    {
        let _ = ui;
        if let Err(e) = crate::common::android::AndroidClipboard::set_text(text) {
            crate::common::logging::error(format!("Failed to copy: {e}"));
        }
    }
    #[cfg(not(target_os = "android"))]
    ui.ctx().copy_text(text.to_owned());
}

pub(crate) fn paste_button(app: &mut RurocoApp, ui: &mut egui::Ui, target: PasteTarget) {
    #[cfg(target_os = "android")]
    {
        let _ = ui;
        match crate::common::android::AndroidClipboard::get_text() {
            Ok(text) => match target {
                PasteTarget::Key => app.key = text,
                PasteTarget::Config => app.commands_config_text = text,
            },
            Err(e) => crate::common::logging::error(format!("Failed to paste: {e}")),
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        app.paste_target = Some(target);
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }
}
