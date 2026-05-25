#![cfg(target_os = "android")]

mod clipboard;
mod clipboard_read;
mod jni_util;
mod keyboard;
mod keyboard_hide;
mod prefs;
mod status_bar;
mod util;

pub(crate) use clipboard::AndroidClipboard;
pub(crate) use keyboard::AndroidKeyboard;
pub(crate) use prefs::AndroidPrefs;
pub(crate) use status_bar::AndroidStatusBar;
pub(crate) use util::AndroidUtil;
