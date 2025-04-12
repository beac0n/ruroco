#![cfg(target_os = "android")]

use crate::ui::run_ui;

#[no_mangle]
unsafe fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).map_err(|e| format!("Could not init slint: {e}")).unwrap();
    run_ui().map_err(|e| format!("Could not run UI: {e}")).unwrap();
}
