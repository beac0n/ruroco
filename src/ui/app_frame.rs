use crate::ui::app::{RurocoApp, Tab};
use crate::ui::tabs;
use eframe::egui;

impl eframe::App for RurocoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.status_bar_dp > 0.0 {
            ui.add_space(self.status_bar_dp);
        }

        #[cfg(all(target_os = "android", feature = "android-build"))]
        if ui.ctx().wants_keyboard_input() {
            let _ = crate::common::android_keyboard::show_soft_keyboard()
                .inspect_err(|e| crate::common::logging::error(format!("{e}")));
        } else {
            let _ = crate::common::android_keyboard::hide_soft_keyboard()
                .inspect_err(|e| crate::common::logging::error(format!("{e}")));
        }

        egui::Panel::top("tabs").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Dashboard, "Dashboard");
                ui.selectable_value(&mut self.active_tab, Tab::Create, "Create");
                ui.selectable_value(&mut self.active_tab, Tab::Execute, "Execute");
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| match self.active_tab {
            Tab::Dashboard => tabs::dashboard::render(self, ui),
            Tab::Create => tabs::create::render(self, ui),
            Tab::Execute => tabs::execute::render(self, ui),
        });
    }
}
