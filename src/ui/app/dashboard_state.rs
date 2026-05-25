#[cfg(target_os = "android")]
use crate::common::logging::error;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum PasteTarget {
    Key,
    Config,
}

pub(crate) struct DashboardState {
    pub(crate) config_text: String,
    pub(crate) key: String,
    pub(crate) show_key: bool,
    pub(crate) paste_target: Option<PasteTarget>,
}

#[cfg(target_os = "android")]
const KEY_PREF: &str = "aes_key";

impl DashboardState {
    pub(crate) fn load_persisted_key() -> String {
        #[cfg(target_os = "android")]
        {
            match crate::common::android::AndroidPrefs::get_string(KEY_PREF) {
                Ok(Some(k)) => k,
                Ok(None) => String::new(),
                Err(e) => {
                    error(format!("Failed to load AES key: {e}"));
                    String::new()
                }
            }
        }
        #[cfg(not(target_os = "android"))]
        String::new()
    }

    #[allow(unused_variables)]
    pub(crate) fn save_key(&mut self, key: String) {
        self.key = key;
        #[cfg(target_os = "android")]
        if let Err(e) = crate::common::android::AndroidPrefs::put_string(KEY_PREF, &self.key) {
            error(format!("Failed to save AES key: {e}"));
        }
    }
}

#[cfg(all(test, feature = "with-gui"))]
mod tests {
    use super::*;

    fn make_state() -> DashboardState {
        DashboardState {
            config_text: String::new(),
            key: String::new(),
            show_key: false,
            paste_target: None,
        }
    }

    #[test]
    fn test_load_persisted_key_empty_on_non_android() {
        assert_eq!(DashboardState::load_persisted_key(), String::new());
    }

    #[test]
    fn test_save_key_sets_key() {
        let mut state = make_state();
        state.save_key("my-key".to_string());
        assert_eq!(state.key, "my-key");
    }
}
