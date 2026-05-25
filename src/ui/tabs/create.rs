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

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use crate::ui::app::CreateForm;
    use crate::ui::saved_command_list::CommandsList;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    fn make_form() -> CreateForm {
        CreateForm {
            address: "127.0.0.1:80".to_string(),
            command: "default".to_string(),
            ip: String::new(),
            permissive: false,
            ipv4: false,
            ipv6: false,
        }
    }

    #[test]
    fn test_render_runs() {
        let dir = tempfile::tempdir().unwrap();
        let mut form = make_form();
        let mut commands_list = CommandsList::create(dir.path());
        let mut config_text = String::new();
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut form, &mut commands_list, &mut config_text, ui);
        });
        harness.run();
    }

    #[test]
    fn test_add_command_click() {
        let dir = tempfile::tempdir().unwrap();
        let state = (make_form(), CommandsList::create(dir.path()), String::new());
        let mut harness = Harness::new_ui_state(
            |ui, (form, cl, config_text): &mut (CreateForm, CommandsList, String)| {
                render(form, cl, config_text, ui);
            },
            state,
        );
        harness.get_by_label("Add Command").click();
        harness.run();
        assert_eq!(harness.state().1.get().len(), 1);
    }
}
