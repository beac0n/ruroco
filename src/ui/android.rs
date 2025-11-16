#![cfg(target_os = "android")]

use crate::ui::run_ui;

#[no_mangle]
unsafe fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Could not init slint");
    run_ui().expect("Could not run UI")
}
