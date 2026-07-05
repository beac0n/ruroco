use crate::ui::app::{DashboardState, PasteTarget};
use eframe::egui;

pub(crate) struct Widgets<'a> {
    ui: &'a mut egui::Ui,
}

impl<'a> Widgets<'a> {
    pub(crate) fn new(ui: &'a mut egui::Ui) -> Self {
        Self { ui }
    }

    pub(crate) fn bordered(color: egui::Color32, inner_margin: f32) -> egui::Frame {
        egui::Frame::default()
            .stroke(egui::Stroke::new(2.0, color))
            .corner_radius(5.0)
            .inner_margin(inner_margin)
    }

    pub(crate) fn icon_button(&mut self, color: egui::Color32, label: &str) -> egui::Response {
        Self::bordered(color, 1.0)
            .show(self.ui, |ui| ui.add_sized([46.0, 46.0], egui::Button::new(label)))
            .inner
    }

    pub(crate) fn equal_buttons(&mut self, labels: &[&str]) -> Vec<bool> {
        let n = labels.len() as f32;
        let btn_w = (self.ui.available_width() - (n - 1.0) * 8.0) / n;
        let mut clicked = vec![false; labels.len()];
        self.ui.horizontal(|ui| {
            for (i, label) in labels.iter().enumerate() {
                clicked[i] = ui.add_sized([btn_w, 50.0], egui::Button::new(*label)).clicked();
            }
        });
        clicked
    }

    pub(crate) fn copy_text(&self, text: &str) {
        #[cfg(target_os = "android")]
        {
            if let Err(e) = crate::common::android::AndroidClipboard::set_text(text) {
                crate::common::logging::error(format!("Failed to copy: {e}"));
            }
        }
        #[cfg(not(target_os = "android"))]
        self.ui.ctx().copy_text(text.to_owned());
    }

    pub(crate) fn paste_button(&mut self, dashboard: &mut DashboardState, target: PasteTarget) {
        #[cfg(target_os = "android")]
        {
            let _ = &self.ui;
            match crate::common::android::AndroidClipboard::get_text() {
                Ok(text) => match target {
                    PasteTarget::Key => dashboard.save_key(text),
                    PasteTarget::Config => dashboard.config_text = text,
                },
                Err(e) => crate::common::logging::error(format!("Failed to paste: {e}")),
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            dashboard.paste_target = Some(target);
            self.ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
    }
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use crate::ui::app::{DashboardState, PasteTarget};
    use egui_kittest::Harness;

    #[test]
    fn test_bordered_constructs() {
        let _ = Widgets::bordered(egui::Color32::RED, 4.0);
    }

    #[test]
    fn test_icon_button_renders() {
        let mut harness = Harness::new_ui(|ui| {
            Widgets::new(ui).icon_button(egui::Color32::BLUE, "▶");
        });
        harness.run();
    }

    #[test]
    fn test_equal_buttons_renders() {
        let mut harness = Harness::new_ui(|ui| {
            let clicked = Widgets::new(ui).equal_buttons(&["A", "B", "C"]);
            assert_eq!(clicked.len(), 3);
        });
        harness.run();
    }

    #[test]
    fn test_copy_text_runs() {
        let mut harness = Harness::new_ui(|ui| {
            Widgets::new(ui).copy_text("hello");
        });
        harness.run();
    }

    #[test]
    fn test_paste_button_sets_target() {
        let mut dashboard = DashboardState {
            config_text: String::new(),
            key: String::new(),
            show_key: false,
            paste_target: None,
        };
        let mut harness = Harness::new_ui(move |ui| {
            Widgets::new(ui).paste_button(&mut dashboard, PasteTarget::Key);
        });
        harness.step();
    }
}
