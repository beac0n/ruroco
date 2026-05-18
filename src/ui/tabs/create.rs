use crate::ui::app::RurocoApp;
use crate::ui::command_data::{add_command_name, CommandData};
use eframe::egui;

pub(crate) fn render(app: &mut RurocoApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(10.0);

        arg_row_text(ui, "server", &mut app.create_address);
        arg_row_text(ui, "command", &mut app.command);
        arg_row_text(ui, "ip sent to server", &mut app.create_ip);
        arg_row(ui, "source IP doesn't have to match provided IP", |ui| {
            ui.checkbox(&mut app.create_permissive, "")
        });
        arg_row(ui, "use ipv4 only", |ui| ui.checkbox(&mut app.create_ipv4, ""));
        arg_row(ui, "use ipv6 only", |ui| ui.checkbox(&mut app.create_ipv6, ""));

        ui.add_space(10.0);

        if ui.add_sized([ui.available_width(), 50.0], egui::Button::new("Add Command")).clicked() {
            let cmd = add_command_name(CommandData {
                address: app.create_address.clone(),
                command: app.command.clone(),
                permissive: app.create_permissive,
                ip: app.create_ip.clone(),
                ipv4: app.create_ipv4,
                ipv6: app.create_ipv6,
                name: String::new(),
            });
            app.commands_list.add(cmd);
            app.sync_config_text();
            app.create_address.clear();
            app.create_ip.clear();
            app.create_permissive = false;
            app.create_ipv4 = false;
            app.create_ipv6 = false;
        }
    });
}

fn arg_row<R>(ui: &mut egui::Ui, label: &str, widget: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let r = ui
        .horizontal(|ui| {
            let w = ui.available_width() * 0.5;
            ui.vertical(|ui| {
                ui.set_width(w);
                ui.add(egui::Label::new(label).wrap());
            });
            widget(ui)
        })
        .inner;
    ui.add_space(6.0);
    r
}

fn arg_row_text(ui: &mut egui::Ui, label: &str, value: &mut String) {
    arg_row(ui, label, |ui| {
        ui.add_sized([ui.available_width(), 50.0], egui::TextEdit::singleline(value))
    });
}
