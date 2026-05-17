use crate::ui::app::{PasteTarget, RurocoApp};
use crate::ui::command_data::command_to_data;
use crate::ui::tabs::widgets;
use eframe::egui;

pub(crate) fn render(app: &mut RurocoApp, ui: &mut egui::Ui, config_height: f32) {
    ui.add_space(10.0);

    egui::ScrollArea::vertical().max_height(config_height).id_salt("config_scroll").show(
        ui,
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.commands_config_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        },
    );

    ui.add_space(6.0);

    let r = widgets::equal_buttons(ui, &["Reset", "💾", "📋", "📥"]);
    if r[0] {
        app.commands_config_text = app.commands_list.to_string();
    }
    if r[1] {
        let cmds: Vec<_> = app.commands_config_text.lines().map(command_to_data).collect();
        app.commands_list.set(cmds);
        app.sync_config_text();
    }
    if r[2] {
        widgets::copy_text(ui, &app.commands_config_text);
    }
    if r[3] {
        widgets::paste_button(app, ui, PasteTarget::Config);
    }
}
