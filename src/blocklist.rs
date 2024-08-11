//! This module is responsible for persisting, holding and checking the blocklist for blocked items

use std::fs;
use std::path::PathBuf;

use log::error;
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use crate::common::get_blocklist_path;

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
    pub fn create(config_dir: &PathBuf) -> Blocklist {
        let blocklist_path = get_blocklist_path(config_dir);
        let blocklist_str =
            fs::read_to_string(&blocklist_path).unwrap_or_else(|_| String::from(""));
        toml::from_str(&blocklist_str).unwrap_or_else(|_| Blocklist {
            list: vec![],
            path: blocklist_path,
        })
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
            Err(e) => return error!("Error serializing blacklist: {e}"),
        };

        match fs::write(&self.path, toml_string) {
            Ok(_) => (),
            Err(e) => error!("Error persisting blacklist: {e}"),
        };
    }
}
