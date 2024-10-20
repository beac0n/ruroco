// Prevent console window in addition to Slint window in Windows release builds when,
// e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ruroco::ui::run_ui;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    run_ui()
}
