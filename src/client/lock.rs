use anyhow::{anyhow, Context};
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
                        return Err(anyhow!("Client already running (lock at {path:?})"));
                    }
                }

                let _ = remove_file(&path);
                Self::open(&path)
                    .with_context(|| format!("Client lock unavailable at {path:?} after cleanup"))?
            }
            Err(e) => {
                return Err(anyhow!("Client lock unavailable at {path:?}: {e}"));
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
