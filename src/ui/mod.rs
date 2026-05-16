#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod android_update;

mod app;
mod colors;
mod command_data;
mod saved_command_list;
mod tabs;

use crate::client::config;
use crate::client::lock::ClientLock;
use std::error::Error;

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
            cc.egui_ctx.set_pixels_per_point(1.5);
            Ok(Box::new(app::RurocoApp::new(&conf_dir)?))
        }),
    )?;
    Ok(())
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
        Box::new(move |_cc| {
            Ok(Box::new(app::RurocoApp::new_with_status_bar(&conf_dir, status_bar_dp)?))
        }),
    )?;
    Ok(())
}
