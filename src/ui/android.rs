#![cfg(target_os = "android")]

use android_activity::AndroidApp;

#[no_mangle]
fn android_main(app: AndroidApp) {
    let status_bar_dp = crate::common::android_status_bar::status_bar_height_dp().unwrap_or(0.0);
    let opts = eframe::NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let _ = crate::ui::run_ui_with_options(opts, status_bar_dp);
}
