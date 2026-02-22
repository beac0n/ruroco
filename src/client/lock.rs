use anyhow::{bail, Context};
use std::fs::{remove_file, File, OpenOptions};
use std::io;
use std::io::Write;
use std::path::PathBuf;

pub(crate) struct ClientLock {
    path: PathBuf,
    file: Option<File>,
}

impl ClientLock {
    pub(crate) fn acquire(path: PathBuf) -> anyhow::Result<Self> {
        let mut file = match Self::open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                if let Some(pid) =
                    std::fs::read_to_string(&path).ok().and_then(|s| s.trim().parse::<u32>().ok())
                {
                    if Self::is_pid_running(pid) {
                        bail!("Client already running (lock at {path:?})");
                    }
                }

                let _ = remove_file(&path);
                Self::open(&path)
                    .with_context(|| format!("Client lock unavailable at {path:?} after cleanup"))?
            }
            Err(e) => {
                bail!("Client lock unavailable at {path:?}: {e}");
            }
        };

        let pid = std::process::id();
        let _ = writeln!(file, "{pid}");

        Ok(Self {
            path,
            file: Some(file),
        })
    }

    fn open(path: &PathBuf) -> io::Result<File> {
        OpenOptions::new().create_new(true).write(true).open(path)
    }

    #[cfg(target_os = "linux")]
    fn is_pid_running(pid: u32) -> bool {
        std::path::Path::new("/proc").join(pid.to_string()).exists()
    }

    #[cfg(target_os = "android")]
    fn is_pid_running(_pid: u32) -> bool {
        false // on android, the app only runs at most once, so it's always "not running"
    }

    #[cfg(target_os = "macos")]
    fn is_pid_running(pid: u32) -> bool {
        std::process::Command::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    fn is_pid_running(pid: u32) -> bool {
        let output =
            std::process::Command::new("tasklist").args(["/FI", &format!("PID eq {pid}")]).output();

        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()),
            Err(_) => false,
        }
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "windows"
    )))]
    fn is_pid_running(_pid: u32) -> bool {
        true // only unknown platforms, we assume that the process is running if there is a file
    }
}

impl Drop for ClientLock {
    fn drop(&mut self) {
        // Close file handle before removing to be Windows-friendly.
        let _ = self.file.take();
        let _ = remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::ClientLock;
    use std::fs;
    use tempfile::TempDir;

    fn temp_lock_path() -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("client.lock");
        (dir, path)
    }

    #[test]
    fn test_acquire_creates_lock_file() {
        let (_dir, path) = temp_lock_path();
        let lock = ClientLock::acquire(path.clone()).unwrap();
        assert!(path.exists());
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.trim().parse::<u32>().is_ok());
        drop(lock);
        assert!(!path.exists());
    }

    #[test]
    fn test_acquire_fails_when_pid_running() {
        let (_dir, path) = temp_lock_path();
        let _lock = ClientLock::acquire(path.clone()).unwrap();
        let result = ClientLock::acquire(path.clone());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("Client already running"), "unexpected error: {err}");
    }

    #[test]
    fn test_acquire_cleans_stale_lock() {
        let (_dir, path) = temp_lock_path();
        // Write a PID that doesn't exist
        fs::write(&path, "999999999\n").unwrap();
        let lock = ClientLock::acquire(path.clone()).unwrap();
        assert!(path.exists());
        drop(lock);
    }

    #[test]
    fn test_acquire_cleans_lock_with_invalid_pid() {
        let (_dir, path) = temp_lock_path();
        // Write invalid content - not a valid PID
        fs::write(&path, "not_a_pid\n").unwrap();
        let lock = ClientLock::acquire(path.clone()).unwrap();
        assert!(path.exists());
        drop(lock);
    }

    #[test]
    fn test_acquire_fails_when_parent_dir_missing() {
        let path = std::path::PathBuf::from("/tmp/no_such_ruroco_dir_xyz/client.lock");
        let result = ClientLock::acquire(path);
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("Client lock unavailable"), "unexpected error: {err}");
    }

    #[test]
    fn test_drop_removes_lock() {
        let (_dir, path) = temp_lock_path();
        let lock = ClientLock::acquire(path.clone()).unwrap();
        assert!(path.exists());
        drop(lock);
        assert!(!path.exists());
    }
}
