//! This module is responsible for persisting, holding and checking the blocklist for blocked items

use anyhow::Context;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::resolve_path;
use crate::server::blocklist_serialization::{deserialize, serialize};
use serde::{Deserialize, Serialize};

/// contains a list of blocked deadlines and a path to where the blocklist is persisted
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Blocklist {
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    map: HashMap<u64, u128>,
    path: PathBuf,
}

impl Blocklist {
    /// create an empty blocklist. Every entry will be saved to config_dir/blocklist.toml.
    /// If the blocklist.toml file already exists, its content will be loaded if possible.
    pub fn create(config_dir: &Path) -> anyhow::Result<Blocklist> {
        let blocklist_path = Self::get_blocklist_path(config_dir);
        let blocklist = if blocklist_path.exists() {
            let blocklist_str = fs::read_to_string(&blocklist_path).with_context(|| {
                format!("Could not read blocklist from path {blocklist_path:?}")
            })?;

            toml::from_str(&blocklist_str).with_context(|| {
                format!("Could not create blocklist from string {blocklist_str}")
            })?
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
        resolve_path(config_dir).join("blocklist.toml")
    }

    /// checks if the provided deadline is within the blocklist
    pub fn is_blocked(&self, key_id: [u8; 8], value: u128) -> bool {
        match self.map.get(&Self::key_id_to_u64(key_id)) {
            Some(v) => v >= &value,
            None => false,
        }
    }

    pub(crate) fn get_counter(&self, key_id: [u8; 8]) -> Option<&u128> {
        self.map.get(&Self::key_id_to_u64(key_id))
    }

    fn key_id_to_u64(key_id: [u8; 8]) -> u64 {
        u64::from_be_bytes(key_id)
    }

    /// returns a reference to the blocklists list
    pub fn get(&self) -> &HashMap<u64, u128> {
        &self.map
    }

    /// adds a new entry to the blocklist
    pub fn add(&mut self, key_id: [u8; 8], entry: u128) {
        self.map.insert(Self::key_id_to_u64(key_id), entry);
    }

    /// saves the current content of the blocklist to the defined path
    pub(crate) fn save(&self) -> anyhow::Result<()> {
        let toml_string = toml::to_string(&self).with_context(|| "Error serializing blacklist")?;

        fs::write(&self.path, toml_string).with_context(|| "Error persisting blacklist")?;

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

        assert_eq!(blocklist.get().get(&Blocklist::key_id_to_u64(key_id)).unwrap().clone(), number);

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
        assert_eq!(blocklist.get().get(&Blocklist::key_id_to_u64(key_id)).unwrap().clone(), 42);

        remove_blocklist();
    }

    #[test]
    fn test_is_blocked() {
        let mut blocklist = create_blocklist();
        let key_id = [0u8; 8];
        blocklist.add(key_id, 42);

        assert!(blocklist.is_blocked(key_id, 42));
        assert!(!blocklist.is_blocked(key_id, 43));

        let mut key_id = [0u8; 8];
        key_id[0] = 1;

        assert!(!blocklist.is_blocked(key_id, 42));

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
        assert!(blocklist.is_blocked(key_id, 100));
        // Counter less than stored value should be blocked
        assert!(blocklist.is_blocked(key_id, 50));
        // Counter greater than stored value should not be blocked
        assert!(!blocklist.is_blocked(key_id, 101));

        remove_blocklist();
    }

    #[test]
    fn test_create_with_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let blocklist = Blocklist::create(dir.path()).unwrap();
        assert!(blocklist.get().is_empty());
        assert!(Blocklist::get_blocklist_path(dir.path()).exists());
    }
}
