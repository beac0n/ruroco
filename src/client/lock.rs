use anyhow::{anyhow, Context};
use nix::fcntl::{Flock, FlockArg};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// Single-instance guard for the client, backed by an exclusive, non-blocking `flock(2)` on a
/// persistent file (never removed) rather than a PID file. Two processes opening this
/// concurrently can never both believe they hold the lock (unlike a create-check-remove-recreate
/// PID file, which races), and a crashed process releases its lock automatically when the kernel
/// closes its file descriptors, so there is no stale-lock state to detect or clean up.
pub(crate) struct ClientLock {
    _lock: Flock<File>,
}

impl ClientLock {
    pub(crate) fn acquire(path: PathBuf) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("Client lock unavailable at {path:?}"))?;

        let lock = Flock::lock(file, FlockArg::LockExclusiveNonblock)
            .map_err(|(_, e)| anyhow!("Client already running (lock at {path:?}): {e}"))?;

        Ok(Self { _lock: lock })
    }
}

#[cfg(test)]
mod tests {
    use super::ClientLock;
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
        drop(lock);
    }

    #[test]
    fn test_acquire_fails_when_already_locked() {
        let (_dir, path) = temp_lock_path();
        let _lock = ClientLock::acquire(path.clone()).unwrap();
        let result = ClientLock::acquire(path.clone());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("Client already running"), "unexpected error: {err}");
    }

    #[test]
    fn test_acquire_succeeds_again_after_drop() {
        let (_dir, path) = temp_lock_path();
        let lock = ClientLock::acquire(path.clone()).unwrap();
        drop(lock);
        // The lock file itself is never removed, but releasing the flock must let the next
        // process acquire it immediately - no stale-lock detection or cleanup involved.
        assert!(path.exists());
        assert!(ClientLock::acquire(path).is_ok());
    }

    #[test]
    fn test_acquire_reuses_preexisting_lock_file() {
        // A lock file left behind by a prior (cleanly exited) run must not block the next one:
        // there is no PID stored in it anymore, only flock state.
        let (_dir, path) = temp_lock_path();
        std::fs::write(&path, "leftover content").unwrap();
        assert!(ClientLock::acquire(path).is_ok());
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
    fn test_second_acquire_fails_while_first_still_held_from_another_thread() {
        // Simulates the race the old PID-file implementation lost: a second acquire arriving
        // while a first is still active must fail outright, never race a stale-lock cleanup path
        // (there is none left) into believing it succeeded too.
        let (_dir, path) = temp_lock_path();
        let path2 = path.clone();

        let held = std::sync::Arc::new(std::sync::Barrier::new(2));
        let released = std::sync::Arc::new(std::sync::Barrier::new(2));
        let (held1, released1) = (held.clone(), released.clone());

        let holder = std::thread::spawn(move || {
            let _lock = ClientLock::acquire(path).unwrap();
            held1.wait();
            released1.wait();
        });

        held.wait();
        let result = ClientLock::acquire(path2);
        assert!(result.is_err(), "acquire must fail while the first lock is still held");
        released.wait();
        holder.join().unwrap();
    }
}
