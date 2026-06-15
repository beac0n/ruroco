use crate::common::crypto_handler::CryptoHandler;
use crate::common::ipc::get_commander_unix_socket_path as util_socket_path;
use crate::common::protocol::KEY_ID_SIZE;
use crate::common::{info, resolve_path};
use crate::server::blocklist::Blocklist;
use crate::server::config::ConfigServer;
use anyhow::{anyhow, bail, Context};
use openssl::version::version;
use std::collections::HashMap;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;

impl ConfigServer {
    pub(crate) fn create_blocklist(&self) -> anyhow::Result<Blocklist> {
        // Blocklist lives in `blocklist_dir` when set (a writable StateDirectory), otherwise in
        // `config_dir`. `Blocklist::get_blocklist_path` resolve_path's a relative dir for us.
        Blocklist::create(self.blocklist_dir.as_ref().unwrap_or(&self.config_dir))
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
        // Socket lives in `socket_dir` when set (a RuntimeDirectory shared with the commander),
        // otherwise in `config_dir`. Both sides must resolve the same path; `util_socket_path`
        // resolve_path's a relative dir for us.
        util_socket_path(self.socket_dir.as_ref().unwrap_or(&self.config_dir))
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

#[cfg(test)]
mod tests {
    use crate::server::config::ConfigServer;
    use std::path::PathBuf;

    #[test]
    fn test_get_key_path() {
        let config_server = ConfigServer {
            config_dir: PathBuf::from("/foo/bar/baz"),
            ..Default::default()
        };

        assert_eq!(
            config_server.get_key_paths().unwrap_err().to_string(),
            r#"Error reading directory "/foo/bar/baz": No such file or directory (os error 2)"#
        );
    }

    #[test]
    fn test_get_key_paths_no_key_files() {
        let dir = tempfile::tempdir().unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = config.get_key_paths();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not find any .key files"));
    }

    #[test]
    fn test_get_key_paths_with_key_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.key"), "key_content").unwrap();
        std::fs::write(dir.path().join("test.txt"), "not_a_key").unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let paths = config.get_key_paths().unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].extension().unwrap() == "key");
    }

    #[test]
    fn test_create_crypto_handlers_duplicate_keys() {
        let dir = tempfile::tempdir().unwrap();
        let content = "duplicate_key_content";
        std::fs::write(dir.path().join("a.key"), content).unwrap();
        std::fs::write(dir.path().join("b.key"), content).unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let err = config.create_crypto_handlers().unwrap_err().to_string();
        assert!(err.contains("Duplicate key files detected"), "unexpected: {err}");
    }

    #[test]
    fn test_create_blocklist() {
        let dir = tempfile::tempdir().unwrap();
        let config = ConfigServer {
            config_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let blocklist = config.create_blocklist().unwrap();
        assert!(blocklist.get().is_empty());
    }

    #[test]
    fn test_get_commander_unix_socket_path() {
        let config = ConfigServer {
            config_dir: PathBuf::from("/tmp/ruroco_test"),
            ..Default::default()
        };
        let path = config.get_commander_unix_socket_path();
        assert!(path.to_str().unwrap().contains("ruroco.socket"));
    }
}
