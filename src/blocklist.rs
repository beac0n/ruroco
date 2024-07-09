use std::fs;
use std::path::PathBuf;

use log::error;
use serde::{Deserialize, Serialize};

use crate::common::get_blocklist_path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Blocklist {
    list: Vec<String>,
    path: PathBuf,
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

    pub fn get(&self) -> Vec<u128> {
        let zero: u128 = 0;
        self.list
            .iter()
            .map(|s| s.parse::<u128>().unwrap_or(0))
            .filter(|deadline| deadline > &zero)
            .collect()
    }

    pub fn add(&mut self, entry: u128) {
        self.list.push(entry.to_string());
    }

    pub fn clean(&mut self, before: u128) {
        self.list.retain(|deadline| deadline > &before.to_string())
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
