use std::fs;
use std::path::PathBuf;

use log::error;
use serde::{Deserialize, Serialize};
use serde::ser::{Serializer, SerializeSeq};

use crate::common::get_blocklist_path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Blocklist {
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    list: Vec<u128>,
    path: PathBuf,
}

fn serialize<S>(vec: &Vec<u128>, serializer: S) -> Result<S::Ok, S::Error>
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

fn deserialize<'d, D>(deserializer: D) -> Result<Vec<u128>, D::Error>
where
    D: serde::Deserializer<'d>,
{
    let vec_str: Vec<String> = serde::Deserialize::deserialize(deserializer)?;
    let vec_u128: Result<Vec<u128>, _> = vec_str.iter().map(|s| s.parse::<u128>()).collect();
    vec_u128.map_err(serde::de::Error::custom)
}

impl Blocklist {
    pub fn create(config_dir: &PathBuf) -> Blocklist {
        let blocklist_path = get_blocklist_path(config_dir);
        let blocklist_str =
            fs::read_to_string(&blocklist_path).unwrap_or_else(|_| String::from(""));
        toml::from_str(&blocklist_str).unwrap_or_else(|_| Blocklist {
            list: vec![],
            path: blocklist_path,
        })
    }

    pub fn is_blocked(&self, deadline: u128) -> bool {
        self.list.contains(&deadline)
    }

    pub fn get(&self) -> &Vec<u128> {
        &self.list
    }

    pub fn add(&mut self, entry: u128) {
        self.list.push(entry);
    }

    pub fn clean(&mut self, before: u128) {
        self.list.retain(|deadline| deadline > &before)
    }

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
