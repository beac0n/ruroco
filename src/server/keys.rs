use crate::common::crypto_handler::CryptoHandler;
use crate::common::protocol::KEY_ID_SIZE;
use crate::common::{info, resolve_path};
use crate::server::blocklist::Blocklist;
use crate::server::config::ConfigServer;
use crate::server::util::get_commander_unix_socket_path as util_socket_path;
use anyhow::{anyhow, bail, Context};
use openssl::version::version;
use std::collections::HashMap;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;

impl ConfigServer {
    pub(crate) fn create_blocklist(&self) -> anyhow::Result<Blocklist> {
        Blocklist::create(&self.resolve_config_dir())
    }

    pub(crate) fn create_crypto_handlers(
        &self,
    ) -> anyhow::Result<HashMap<[u8; KEY_ID_SIZE], CryptoHandler>> {
        let key_paths = self.get_key_paths()?;
        info(format!("Creating server, loading keys from {key_paths:?}, using {} ...", version()));

        let content_to_path = Self::get_content_to_path(&key_paths)?;
        if key_paths.len() != content_to_path.len() {
            bail!("Duplicate key files detected; refusing to start");
        }

        content_to_path
            .into_iter()
            .map(|(content, p)| {
                let h = CryptoHandler::create(&content)
                    .with_context(|| format!("load key {}", p.display()))?;
                info(format!("loading key with id {:X?}", &h.id));
                Ok((h.id, h))
            })
            .collect()
    }

    pub(crate) fn get_commander_unix_socket_path(&self) -> PathBuf {
        util_socket_path(&self.resolve_config_dir())
    }

    pub(crate) fn resolve_config_dir(&self) -> PathBuf {
        resolve_path(&self.config_dir)
    }

    fn get_content_to_path(key_paths: &[PathBuf]) -> anyhow::Result<HashMap<String, PathBuf>> {
        key_paths
            .iter()
            .map(|p| {
                fs::read_to_string(p)
                    .with_context(|| format!("Could not read key file {}", p.display()))
                    .map(|content| (content, p.clone()))
            })
            .collect::<anyhow::Result<HashMap<String, PathBuf>>>()
    }

    pub(crate) fn get_key_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let config_dir = self.resolve_config_dir();

        let entries: ReadDir = match fs::read_dir(&config_dir) {
            Ok(entries) => entries,
            Err(e) => bail!("Error reading directory {config_dir:?}: {e}"),
        };

        let key_files: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "key"))
            .collect();

        match key_files.len() {
            0 => Err(anyhow!("Could not find any .key files in {config_dir:?}")),
            _ => Ok(key_files),
        }
    }
}
