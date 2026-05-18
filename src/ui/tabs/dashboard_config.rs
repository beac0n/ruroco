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
