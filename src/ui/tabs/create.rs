use crate::ui::app::CreateForm;
use crate::ui::command_data::{add_command_name, CommandData};
use crate::ui::saved_command_list::CommandsList;
use eframe::egui;

pub(crate) fn render(
    form: &mut CreateForm,
    commands_list: &mut CommandsList,
    config_text: &mut String,
    ui: &mut egui::Ui,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(10.0);

        arg_row_text(ui, "server", &mut form.address);
        arg_row_text(ui, "command", &mut form.command);
        arg_row_text(ui, "ip sent to server", &mut form.ip);
        arg_row(ui, "source IP doesn't have to match provided IP", |ui| {
            ui.checkbox(&mut form.permissive, "")
        });
        arg_row(ui, "use ipv4 only", |ui| ui.checkbox(&mut form.ipv4, ""));
        arg_row(ui, "use ipv6 only", |ui| ui.checkbox(&mut form.ipv6, ""));

        ui.add_space(10.0);

        if ui.add_sized([ui.available_width(), 50.0], egui::Button::new("Add Command")).clicked() {
            let cmd = add_command_name(CommandData {
                address: form.address.clone(),
                command: form.command.clone(),
                permissive: form.permissive,
                ip: form.ip.clone(),
                ipv4: form.ipv4,
                ipv6: form.ipv6,
                name: String::new(),
            });
            commands_list.add(cmd);
            *config_text = commands_list.to_string();
            form.address.clear();
            form.ip.clear();
            form.permissive = false;
            form.ipv4 = false;
            form.ipv6 = false;
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
