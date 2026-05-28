use crate::client::counter::Counter;
use crate::client::send::Sender;
use crate::common::crypto_handler::CryptoHandler;
use crate::common::logging::error;
use crate::common::{info, now_nanos};
use crate::ui::app::{DashboardState, PasteTarget};
use crate::ui::tabs::widgets;
use eframe::egui;

pub(crate) fn render(dashboard: &mut DashboardState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("AES Key:");
        ui.add(
            egui::TextEdit::singleline(&mut dashboard.key)
                .hint_text("Generate or paste your key here")
                .password(!dashboard.show_key)
                .desired_width(f32::INFINITY),
        );
    });

    ui.add_space(6.0);

    let lock_label = if dashboard.show_key { "🔒" } else { "🔓" };
    let r = widgets::Widgets::new(ui).equal_buttons(&["Generate", lock_label, "📋", "📥"]);
    if r[0] {
        match CryptoHandler::gen_key() {
            Ok(k) => dashboard.save_key(k),
            Err(e) => error(format!("Failed to generate key: {e}")),
        }
    }
    if r[1] {
        dashboard.show_key = !dashboard.show_key;
    }
    if r[2] {
        widgets::Widgets::new(ui).copy_text(&dashboard.key);
    }
    if r[3] {
        widgets::Widgets::new(ui).paste_button(dashboard, PasteTarget::Key);
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);

    if widgets::Widgets::new(ui).equal_buttons(&["Reseed Counter"])[0] {
        match Sender::get_counter_path()
            .and_then(|p| now_nanos().and_then(|v| Counter::reseed(p, v)))
        {
            Ok(()) => info("Counter reseeded"),
            Err(e) => error(format!("Failed to reseed counter: {e}")),
        }
    }
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    fn make_state() -> DashboardState {
        DashboardState {
            config_text: String::new(),
            key: String::new(),
            show_key: false,
            paste_target: None,
        }
    }

    #[test]
    fn test_render_runs() {
        let mut dashboard = make_state();
        let mut harness = Harness::new_ui(move |ui| {
            render(&mut dashboard, ui);
        });
        harness.run();
    }

    #[test]
    fn test_generate_key_populates_key() {
        let mut harness = Harness::new_ui_state(
            |ui, dashboard: &mut DashboardState| {
                render(dashboard, ui);
            },
            make_state(),
        );
        harness.get_by_label("Generate").click();
        harness.run();
        assert!(!harness.state().key.is_empty(), "key should be populated after Generate click");
    }

    #[test]
    fn test_toggle_show_key() {
        let mut harness = Harness::new_ui_state(
            |ui, dashboard: &mut DashboardState| {
                render(dashboard, ui);
            },
            make_state(),
        );
        assert!(!harness.state().show_key);
        harness.get_by_label("🔓").click();
        harness.run();
        assert!(harness.state().show_key);
    }

    #[test]
    fn test_copy_key_button() {
        let mut state = make_state();
        state.key = "test-key-value".to_string();
        let mut harness = Harness::new_ui_state(
            |ui, dashboard: &mut DashboardState| {
                render(dashboard, ui);
            },
            state,
        );
        harness.get_by_label("📋").click();
        harness.run();
    }

    #[test]
    fn test_paste_key_button() {
        let mut harness = Harness::new_ui_state(
            |ui, dashboard: &mut DashboardState| {
                render(dashboard, ui);
            },
            make_state(),
        );
        harness.get_by_label("📥").click();
        harness.step();
    }

    #[test]
    fn test_reseed_counter_button() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("RUROCO_CONF_DIR", dir.path());
        let mut harness = Harness::new_ui_state(
            |ui, dashboard: &mut DashboardState| {
                render(dashboard, ui);
            },
            make_state(),
        );
        harness.get_by_label("Reseed Counter").click();
        harness.run();
        std::env::remove_var("RUROCO_CONF_DIR");
        assert!(dir.path().join("counter").exists(), "counter file should exist after reseed");
    }
}
