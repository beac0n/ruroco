#![cfg(target_os = "android")]

use crate::client::update::Updater;
use crate::common::info;
use android_activity::AndroidApp;
use anyhow::anyhow;

#[no_mangle]
fn android_main(app: AndroidApp) {
    let status_bar_dp = crate::common::android::AndroidStatusBar::height_dp().unwrap_or(0.0);
    let opts = eframe::NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let _ = crate::ui::run_ui_with_options(opts, status_bar_dp);
}

pub(crate) fn update_android() -> anyhow::Result<()> {
    let data = Updater::get_github_api_data(None)?;
    let asset = data
        .assets
        .into_iter()
        .find(|a| a.name.ends_with(".apk"))
        .ok_or_else(|| anyhow!("No APK asset found in latest release"))?;

    let util = crate::common::android::AndroidUtil::create()?;
    let uri = util.uri_parse(asset.browser_download_url)?;
    let intent = util.new_view_intent(&uri)?;
    let result = util.start_activity(&intent);
    let _ = result.inspect_err(|err| {
        info(format!("Error (prob. expected) when opening browser window: {err}"))
    });

    Ok(())
}
