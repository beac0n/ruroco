use crate::common::error;
use crate::ui::run_ui;
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();

    run_ui().unwrap_or_else(|e| error(&format!("Could not run ui: {e}")));
}
