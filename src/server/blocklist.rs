//! This module is responsible for persisting, holding, and checking the blocklist for blocked items

use anyhow::Context;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::fs::write_atomic;
use crate::common::protocol::KEY_ID_SIZE;
use crate::common::resolve_path;
use serde::{Deserialize, Serialize};

/// contains a list of blocked deadlines and a path to where the blocklist is persisted.
///
/// Stability: the on-disk format is msgpack of this struct, so any incompatible schema change
/// makes `rmp_serde::from_slice` fail (surfaced as "Could not create blocklist from vec"), i.e.
/// it already fails closed without an explicit version marker. This is local server state, not a
/// cross-version wire contract, so no version field is carried.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Blocklist {
    map: HashMap<[u8; KEY_ID_SIZE], u128>,
    path: PathBuf,
}

impl Blocklist {
    /// Create an empty blocklist. Every entry will be saved to config_dir/blocklist.msgpck.
    /// If the blocklist.msgpck file already exists, its content will be loaded if possible.
    pub fn create(config_dir: &Path) -> anyhow::Result<Blocklist> {
        let blocklist_path = Self::get_blocklist_path(config_dir);
        let blocklist = if blocklist_path.exists() {
            let blocklist_str = fs::read(&blocklist_path).with_context(|| {
                format!("Could not read blocklist from path {blocklist_path:?}")
            })?;

            rmp_serde::from_slice(&blocklist_str)
                .with_context(|| "Could not create blocklist from vec")?
        } else {
            Blocklist {
                map: HashMap::new(),
                path: blocklist_path,
            }
        };

        blocklist.save()?;
        Ok(blocklist)
    }

    pub fn get_blocklist_path(config_dir: &Path) -> PathBuf {
        resolve_path(config_dir).join("blocklist.msgpck")
    }

    /// Returns `true` if this `(key_id, counter)` pair has already been accepted.
    ///
    /// Uses `>=`: a counter equal to the stored value is a replay, because the
    /// stored value records the most recent counter accepted. Do not relax this
    /// to `>` — identical packets (retransmits, captures, adversarial replays)
    /// must be rejected.
    pub(crate) fn is_counter_replayed(&self, key_id: [u8; KEY_ID_SIZE], value: u128) -> bool {
        match self.map.get(&key_id) {
            Some(v) => v >= &value,
            None => true,
        }
    }

    pub(crate) fn seed_if_absent(&mut self, key_id: [u8; KEY_ID_SIZE], floor: u128) {
        self.map.entry(key_id).or_insert(floor);
    }

    pub(crate) fn get_counter(&self, key_id: [u8; KEY_ID_SIZE]) -> Option<&u128> {
        self.map.get(&key_id)
    }

    pub fn get(&self) -> &HashMap<[u8; KEY_ID_SIZE], u128> {
        &self.map
    }

    pub(crate) fn add(&mut self, key_id: [u8; KEY_ID_SIZE], entry: u128) {
        self.map.insert(key_id, entry);
    }

    pub(crate) fn save(&self) -> anyhow::Result<()> {
        let vec = rmp_serde::to_vec(&self).with_context(|| "Error serializing blacklist")?;
        write_atomic(&self.path, vec.as_slice()).with_context(|| "Error persisting blacklist")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use crate::server::blocklist::Blocklist;

    fn create_blocklist() -> Blocklist {
        remove_blocklist();
        Blocklist::create(&env::current_dir().unwrap()).unwrap()
    }

    fn remove_blocklist() {
        let blocklist_path = Blocklist::get_blocklist_path(&env::current_dir().unwrap());
        let _ = fs::remove_file(&blocklist_path);
    }

    #[test]
    fn test_add() {
        let mut blocklist = create_blocklist();
        let key_id = [0u8; 8];
        let number: u128 = 42;

        blocklist.add(key_id, number);
        assert_eq!(blocklist.get().len(), 1);

        assert_eq!(blocklist.get().get(&key_id).unwrap().clone(), number);

        remove_blocklist();
    }

    #[test]
    fn test_save() {
        let mut blocklist = create_blocklist();

        let key_id = [0u8; 8];
        blocklist.add(key_id, 42);
        blocklist.save().unwrap();

        let other_blocklist = Blocklist::create(&env::current_dir().unwrap()).unwrap();
        assert_eq!(other_blocklist.get().len(), 1);
        assert_eq!(blocklist.get().get(&key_id).unwrap().clone(), 42);

        remove_blocklist();
    }

    #[test]
    fn test_is_blocked() {
        let mut blocklist = create_blocklist();
        let key_id = [0u8; 8];
        blocklist.add(key_id, 42);

        assert!(blocklist.is_counter_replayed(key_id, 42));
        assert!(!blocklist.is_counter_replayed(key_id, 43));

        let mut key_id = [0u8; 8];
        key_id[0] = 1;

        assert!(blocklist.is_counter_replayed(key_id, 42));

        remove_blocklist();
    }

    #[test]
    fn test_get_counter() {
        let mut blocklist = create_blocklist();
        let key_id = [0u8; 8];
        assert_eq!(blocklist.get_counter(key_id), None);

        blocklist.add(key_id, 100);
        assert_eq!(blocklist.get_counter(key_id), Some(&100));

        let unknown_key_id = [1u8; 8];
        assert_eq!(blocklist.get_counter(unknown_key_id), None);

        remove_blocklist();
    }

    #[test]
    fn test_is_blocked_lower_counter() {
        let mut blocklist = create_blocklist();
        let key_id = [0u8; 8];
        blocklist.add(key_id, 100);

        // Counter equal to stored value should be blocked
        assert!(blocklist.is_counter_replayed(key_id, 100));
        // Counter less than stored value should be blocked
        assert!(blocklist.is_counter_replayed(key_id, 50));
        // Counter greater than stored value should not be blocked
        assert!(!blocklist.is_counter_replayed(key_id, 101));

        remove_blocklist();
    }

    #[test]
    fn test_create_with_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let blocklist = Blocklist::create(dir.path()).unwrap();
        assert!(blocklist.get().is_empty());
        assert!(Blocklist::get_blocklist_path(dir.path()).exists());
    }

    #[test]
    fn test_seed_if_absent() {
        let dir = tempfile::tempdir().unwrap();
        let mut blocklist = Blocklist::create(dir.path()).unwrap();
        let key_id = [1u8; 8];

        blocklist.seed_if_absent(key_id, 50);
        assert_eq!(blocklist.get_counter(key_id), Some(&50));

        // Second call with different floor must not overwrite
        blocklist.seed_if_absent(key_id, 999);
        assert_eq!(blocklist.get_counter(key_id), Some(&50));
    }

    #[test]
    fn test_create_with_corrupted_blocklist_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = Blocklist::get_blocklist_path(dir.path());
        fs::write(&path, b"this is not valid msgpack data").unwrap();
        let result = Blocklist::create(dir.path());
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("Could not create blocklist from vec"),
            "unexpected error"
        );
    }

    #[test]
    fn test_create_with_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let _blocklist = Blocklist::create(dir.path()).unwrap();
        let path = Blocklist::get_blocklist_path(dir.path());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        let result = Blocklist::create(dir.path());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        if !nix::unistd::getuid().is_root() {
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Could not read blocklist"));
        }
    }

    #[test]
    fn test_save_fails_on_readonly_dir() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let blocklist = Blocklist::create(dir.path()).unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let result = blocklist.save();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        if !nix::unistd::getuid().is_root() {
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Error persisting blacklist"));
        }
    }
}
