#[cfg(target_os = "android")]
mod android;

mod app;
mod app_frame;
mod colors;
mod command_data;
mod saved_command_list;
mod tabs;

use crate::client::config;
use crate::client::lock::ClientLock;
use std::error::Error;

fn set_font_size(ctx: &eframe::egui::Context, size: f32) {
    let mut style = (*ctx.global_style()).clone();
    for font_id in style.text_styles.values_mut() {
        font_id.size = size;
    }
    ctx.set_global_style(style);
}

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let conf_dir = config::get_conf_dir()?;
    let _lock = ClientLock::acquire(conf_dir.join("client.lock"))?;
    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([540.0, 1200.0])
            .with_title("ruroco"),
        ..Default::default()
    };
    eframe::run_native(
        "ruroco",
        opts,
        Box::new(move |cc| {
            set_font_size(&cc.egui_ctx, 14.0);
            Ok(Box::new(app::RurocoApp::new(&conf_dir)?))
        }),
    )?;
    Ok(())
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::set_font_size;
    use egui_kittest::Harness;

    #[test]
    fn test_set_font_size() {
        let mut harness = Harness::new_ui(|ui| {
            set_font_size(ui.ctx(), 18.0);
        });
        harness.run();
    }
}

#[cfg(target_os = "android")]
pub fn run_ui_with_options(
    opts: eframe::NativeOptions,
    status_bar_dp: f32,
) -> Result<(), Box<dyn Error>> {
    let conf_dir = config::get_conf_dir()?;
    let _lock = ClientLock::acquire(conf_dir.join("client.lock"))?;
    eframe::run_native(
        "ruroco",
        opts,
        Box::new(move |cc| {
            set_font_size(&cc.egui_ctx, 14.0);
            Ok(Box::new(app::RurocoApp::new_with_status_bar(&conf_dir, status_bar_dp)?))
        }),
    )?;
    Ok(())
}
