#![cfg(target_os = "android")]

use crate::client::update::Updater;
use crate::common::info;
pub fn update_android() -> Result<(), String> {
    let data = Updater::get_github_api_data(None)?;
    let asset = data
        .assets
        .into_iter()
        .find(|a| a.name.ends_with(".apk"))
        .ok_or(Err("No APK asset found in latest release"))?;

    let util = crate::ui::android_util::AndroidUtil::create()?;
    let uri = util.uri_parse(asset.browser_download_url)?;
    let intent = util.new_view_intent(&uri)?;
    let result = util.start_activity(&intent);
    let _ = result.inspect_err(|err| {
        info(&format!("Error (prob. expected) when opening browser window: {err}"))
    });

    Ok(())
}
