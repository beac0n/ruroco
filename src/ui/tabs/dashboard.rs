#[cfg(target_os = "linux")]
use crate::client::update::Updater;
use crate::common::logging::error;
#[cfg(target_os = "android")]
use crate::ui::android::update_android;
use crate::ui::app::{DashboardState, PasteTarget};
use crate::ui::saved_command_list::CommandsList;
use eframe::egui;

pub(crate) fn render(
    dashboard: &mut DashboardState,
    commands_list: &mut CommandsList,
    ui: &mut egui::Ui,
) {
    if let Some(text) = ui.ctx().input(|i| {
        i.events.iter().find_map(|e| {
            if let egui::Event::Paste(t) = e {
                Some(t.clone())
            } else {
                None
            }
        })
    }) {
        if let Some(target) = dashboard.paste_target.take() {
            match target {
                PasteTarget::Key => dashboard.save_key(text),
                PasteTarget::Config => dashboard.config_text = text,
            }
        }
    }

    let config_height = ui.available_height() * 0.45;
    super::dashboard_config::render(dashboard, commands_list, ui, config_height);

    ui.separator();
    ui.add_space(6.0);

    super::dashboard_key::render(dashboard, ui);

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

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use crate::ui::saved_command_list::CommandsList;
    use egui_kittest::Harness;

    fn make_state(paste_target: Option<PasteTarget>) -> (DashboardState, CommandsList) {
        let dir = tempfile::tempdir().unwrap();
        (
            DashboardState {
                config_text: String::new(),
                key: String::new(),
                show_key: false,
                paste_target,
            },
            CommandsList::create(dir.path()),
        )
    }

    #[test]
    fn test_render_runs() {
        let dir = tempfile::tempdir().unwrap();
        let mut dashboard = DashboardState {
            config_text: String::new(),
            key: String::new(),
            show_key: false,
            paste_target: None,
        };
        let mut commands_list = CommandsList::create(dir.path());
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut dashboard, &mut commands_list, ui);
        });
        harness.run();
    }

    #[test]
    fn test_paste_event_sets_key() {
        let state = make_state(Some(PasteTarget::Key));
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui);
            },
            state,
        );
        harness.input_mut().events.push(eframe::egui::Event::Paste("pasted-key".to_string()));
        harness.step();
        assert_eq!(harness.state().0.key, "pasted-key");
    }

    #[test]
    fn test_paste_event_sets_config() {
        let state = make_state(Some(PasteTarget::Config));
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui);
            },
            state,
        );
        harness.input_mut().events.push(eframe::egui::Event::Paste("pasted-config".to_string()));
        harness.step();
        assert_eq!(harness.state().0.config_text, "pasted-config");
    }

    #[test]
    fn test_non_paste_event_is_ignored() {
        let state = make_state(None);
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui);
            },
            state,
        );
        harness.input_mut().events.push(eframe::egui::Event::PointerGone);
        harness.step();
        assert!(harness.state().0.paste_target.is_none());
        assert!(harness.state().0.key.is_empty());
    }
}
