use crate::ui::app::{DashboardState, PasteTarget};
use crate::ui::command_data::command_to_data;
use crate::ui::saved_command_list::CommandsList;
use crate::ui::tabs::widgets;
use eframe::egui;

pub(crate) fn render(
    dashboard: &mut DashboardState,
    commands_list: &mut CommandsList,
    ui: &mut egui::Ui,
    config_height: f32,
) {
    ui.add_space(10.0);

    egui::ScrollArea::vertical().max_height(config_height).id_salt("config_scroll").show(
        ui,
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut dashboard.config_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        },
    );

    ui.add_space(6.0);

    let mut w = widgets::Widgets::new(ui);
    let r = w.equal_buttons(&["Reset", "💾", "📋", "📥"]);
    if r[0] {
        dashboard.config_text = commands_list.to_string();
    }
    if r[1] {
        let cmds: Vec<_> = dashboard.config_text.lines().map(command_to_data).collect();
        commands_list.set(cmds);
        dashboard.config_text = commands_list.to_string();
    }
    if r[2] {
        w.copy_text(&dashboard.config_text);
    }
    if r[3] {
        w.paste_button(dashboard, PasteTarget::Config);
    }
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    fn make_state(config: &str) -> DashboardState {
        DashboardState {
            config_text: config.to_string(),
            key: String::new(),
            show_key: false,
            paste_target: None,
        }
    }

    #[test]
    fn test_render_runs() {
        let dir = tempfile::tempdir().unwrap();
        let mut dashboard = make_state("");
        let mut commands_list = CommandsList::create(dir.path());
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut dashboard, &mut commands_list, ui, 200.0);
        });
        harness.run();
    }

    #[test]
    fn test_reset_button_restores_config() {
        let dir = tempfile::tempdir().unwrap();
        let state = (make_state("edited text"), CommandsList::create(dir.path()));
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui, 200.0);
            },
            state,
        );
        harness.get_by_label("Reset").click();
        harness.run();
        // After reset, config_text should match commands_list.to_string() (empty list → "")
        assert_eq!(harness.state().0.config_text, "");
    }

    #[test]
    fn test_save_button_updates_commands() {
        let dir = tempfile::tempdir().unwrap();
        let config_line = "send --address 127.0.0.1:80 --command default";
        let state = (make_state(config_line), CommandsList::create(dir.path()));
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui, 200.0);
            },
            state,
        );
        harness.get_by_label("💾").click();
        harness.run();
        assert_eq!(harness.state().1.get().len(), 1);
    }

    #[test]
    fn test_copy_config_button() {
        let dir = tempfile::tempdir().unwrap();
        let state = (make_state("some config"), CommandsList::create(dir.path()));
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui, 200.0);
            },
            state,
        );
        harness.get_by_label("📋").click();
        harness.run();
    }

    #[test]
    fn test_paste_config_button() {
        let dir = tempfile::tempdir().unwrap();
        let state = (make_state(""), CommandsList::create(dir.path()));
        let mut harness = Harness::new_ui_state(
            |ui, (dashboard, cl): &mut (DashboardState, CommandsList)| {
                render(dashboard, cl, ui, 200.0);
            },
            state,
        );
        harness.get_by_label("📥").click();
        harness.step();
    }
}
