use anyhow::{anyhow, Context};
use nix::fcntl::{Flock, FlockArg};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// Generic single-instance guard backed by an exclusive, non-blocking `flock(2)` on a persistent
/// file (never removed) rather than a PID file. Two processes opening this concurrently can never
/// both believe they hold the lock (unlike a create-check-remove-recreate PID file, which races),
/// and a crashed process releases its lock automatically when the kernel closes its file
/// descriptors, so there is no stale-lock state to detect or clean up.
#[derive(Debug)]
pub(crate) struct InstanceLock {
    _lock: Flock<File>,
}

impl InstanceLock {
    /// `already_running_msg` is folded into the error when another instance already holds `path`,
    /// e.g. "Client already running" or "Commander already running".
    pub(crate) fn acquire(path: PathBuf, already_running_msg: &str) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("Lock file unavailable at {path:?}"))?;

        let lock = Flock::lock(file, FlockArg::LockExclusiveNonblock)
            .map_err(|(_, e)| anyhow!("{already_running_msg} (lock at {path:?}): {e}"))?;

        Ok(Self { _lock: lock })
    }
}

#[cfg(test)]
mod tests {
    use super::InstanceLock;
    use tempfile::TempDir;

    fn temp_lock_path() -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("test.lock");
        (dir, path)
    }

    #[test]
    fn test_acquire_creates_lock_file() {
        let (_dir, path) = temp_lock_path();
        let lock = InstanceLock::acquire(path.clone(), "Already running").unwrap();
        assert!(path.exists());
        drop(lock);
    }

    #[test]
    fn test_acquire_fails_when_already_locked() {
        let (_dir, path) = temp_lock_path();
        let _lock = InstanceLock::acquire(path.clone(), "Already running").unwrap();
        let result = InstanceLock::acquire(path.clone(), "Already running");
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("Already running"), "unexpected error: {err}");
    }

    #[test]
    fn test_acquire_succeeds_again_after_drop() {
        let (_dir, path) = temp_lock_path();
        let lock = InstanceLock::acquire(path.clone(), "Already running").unwrap();
        drop(lock);
        // The lock file itself is never removed, but releasing the flock must let the next
        // process acquire it immediately - no stale-lock detection or cleanup involved.
        assert!(path.exists());
        assert!(InstanceLock::acquire(path, "Already running").is_ok());
    }

    #[test]
    fn test_acquire_reuses_preexisting_lock_file() {
        // A lock file left behind by a prior (cleanly exited) run must not block the next one:
        // there is no PID stored in it anymore, only flock state.
        let (_dir, path) = temp_lock_path();
        std::fs::write(&path, "leftover content").unwrap();
        assert!(InstanceLock::acquire(path, "Already running").is_ok());
    }

    #[test]
    fn test_acquire_fails_when_parent_dir_missing() {
        let path = std::path::PathBuf::from("/tmp/no_such_ruroco_dir_xyz/test.lock");
        let result = InstanceLock::acquire(path, "Already running");
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("Lock file unavailable"), "unexpected error: {err}");
    }

    #[test]
    fn test_second_acquire_fails_while_first_still_held_from_another_thread() {
        // Simulates the race a PID-file implementation would lose: a second acquire arriving
        // while a first is still active must fail outright, never race a stale-lock cleanup path
        // into believing it succeeded too.
        let (_dir, path) = temp_lock_path();
        let path2 = path.clone();

        let held = std::sync::Arc::new(std::sync::Barrier::new(2));
        let released = std::sync::Arc::new(std::sync::Barrier::new(2));
        let (held1, released1) = (held.clone(), released.clone());

        let holder = std::thread::spawn(move || {
            let _lock = InstanceLock::acquire(path, "Already running").unwrap();
            held1.wait();
            released1.wait();
        });

        held.wait();
        let result = InstanceLock::acquire(path2, "Already running");
        assert!(result.is_err(), "acquire must fail while the first lock is still held");
        released.wait();
        holder.join().unwrap();
    }
}
