//! This module is responsible for persisting, holding and checking the blocklist for blocked items

use std::fs;
use std::path::{Path, PathBuf};

use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use crate::common::{error, resolve_path};

/// contains a list of blocked deadlines and a path to where the blocklist is persisted
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Blocklist {
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    list: Vec<u128>,
    path: PathBuf,
}

/// u128 is not supported by toml, so we have to serialize by saving them as strings
fn serialize<S>(vec: &[u128], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let vec_str: Vec<String> = vec.iter().map(|u| u.to_string()).collect();
    let mut seq = serializer.serialize_seq(Some(vec_str.len()))?;
    for element in vec_str {
        seq.serialize_element(&element)?;
    }
    seq.end()
}

/// u128 is not supported by toml, so we have to deserialize by parsing u128 from string
fn deserialize<'d, D>(deserializer: D) -> Result<Vec<u128>, D::Error>
where
    D: serde::Deserializer<'d>,
{
    let vec_str: Vec<String> = serde::Deserialize::deserialize(deserializer)?;
    let vec_u128: Result<Vec<u128>, _> = vec_str.iter().map(|s| s.parse::<u128>()).collect();
    vec_u128.map_err(serde::de::Error::custom)
}

impl Blocklist {
    /// create an empty blocklist. Every entry will be saved to config_dir/blocklist.toml.
    /// If the blocklist.toml file already exists, its content will be loaded if possible.
    pub fn create(config_dir: &Path) -> Blocklist {
        let blocklist_path = Self::get_blocklist_path(config_dir);
        let blocklist_str = fs::read_to_string(&blocklist_path).unwrap_or_else(|_| "".to_string());
        toml::from_str(&blocklist_str).unwrap_or_else(|_| Blocklist {
            list: vec![],
            path: blocklist_path,
        })
    }

    fn get_blocklist_path(config_dir: &Path) -> PathBuf {
        resolve_path(config_dir).join("blocklist.toml")
    }

    pub fn delete_blocklist_file(config_dir: &Path) {
        let blocklist_path = Self::get_blocklist_path(config_dir);
        let _ = fs::remove_file(&blocklist_path);
    }

    /// checks if the provided deadline is within the blocklist
    pub fn is_blocked(&self, deadline: u128) -> bool {
        self.list.contains(&deadline)
    }

    /// returns a reference to the blocklists list
    pub fn get(&self) -> &Vec<u128> {
        &self.list
    }

    /// adds a new entry to the blocklist
    pub fn add(&mut self, entry: u128) {
        self.list.push(entry);
    }

    /// removes every entry in the blocklist that is smaller or equal to before
    pub fn clean(&mut self, before: u128) {
        self.list.retain(|deadline| deadline > &before)
    }

    /// saves the current content of the blocklist to the defined path
    pub fn save(&self) {
        let toml_string = match toml::to_string(&self) {
            Ok(s) => s,
            Err(e) => return error(&format!("Error serializing blacklist: {e}")),
        };

        match fs::write(&self.path, toml_string) {
            Ok(_) => (),
            Err(e) => error(&format!("Error persisting blacklist: {e}")),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::server::blocklist::Blocklist;

    fn create_blocklist() -> Blocklist {
        remove_blocklist();
        Blocklist::create(&env::current_dir().unwrap())
    }

    fn remove_blocklist() {
        Blocklist::delete_blocklist_file(&env::current_dir().unwrap())
    }

    #[test]
    fn test_add() {
        let mut blocklist = create_blocklist();
        let number: u128 = 42;
        let another_number: u128 = 1337;

        blocklist.add(number);
        assert_eq!(blocklist.get().len(), 1);
        assert_eq!(blocklist.get().first().unwrap().clone(), number);

        blocklist.add(another_number);
        assert_eq!(blocklist.get().len(), 2);
        assert_eq!(blocklist.get().first().unwrap().clone(), number);
        assert_eq!(blocklist.get().get(1).unwrap().clone(), another_number);

        remove_blocklist();
    }

    #[test]
    fn test_clean() {
        let mut blocklist = create_blocklist();

        blocklist.add(21);
        blocklist.add(42);
        blocklist.add(63);
        blocklist.add(84);
        blocklist.add(105);

        assert_eq!(blocklist.get().len(), 5);

        blocklist.clean(63);
        assert_eq!(blocklist.get().len(), 2);
        assert_eq!(blocklist.get().first().unwrap().clone(), 84);
        assert_eq!(blocklist.get().get(1).unwrap().clone(), 105);

        remove_blocklist();
    }

    #[test]
    fn test_save() {
        let mut blocklist = create_blocklist();

        blocklist.add(42);
        blocklist.save();
        blocklist.add(1337);

        let other_blocklist = Blocklist::create(&env::current_dir().unwrap());
        assert_eq!(other_blocklist.get().len(), 1);
        assert_eq!(other_blocklist.get().first().unwrap().clone(), 42);

        remove_blocklist();
    }

    #[test]
    fn test_is_blocked() {
        let mut blocklist = create_blocklist();

        blocklist.add(42);

        assert!(blocklist.is_blocked(42));
        assert!(!blocklist.is_blocked(1337));

        remove_blocklist();
    }
}
