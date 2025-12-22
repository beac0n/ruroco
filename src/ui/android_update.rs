#![cfg(target_os = "android")]

use crate::client::update::Updater;
use crate::common::info;
use anyhow::anyhow;
pub(crate) fn update_android() -> anyhow::Result<()> {
    let data = Updater::get_github_api_data(None)?;
    let asset = data
        .assets
        .into_iter()
        .find(|a| a.name.ends_with(".apk"))
        .ok_or_else(|| anyhow!("No APK asset found in latest release"))?;

    let util = crate::common::android_util::AndroidUtil::create()?;
    let uri = util.uri_parse(asset.browser_download_url)?;
    let intent = util.new_view_intent(&uri)?;
    let result = util.start_activity(&intent);
    let _ = result.inspect_err(|err| {
        info(&format!("Error (prob. expected) when opening browser window: {err}"))
    });

    Ok(())
}
