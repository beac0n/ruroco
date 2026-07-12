use crate::common::instance_lock::InstanceLock;
use std::path::PathBuf;

/// Single-instance guard for the client. See `InstanceLock` for the locking mechanism.
pub(crate) struct ClientLock {
    _lock: InstanceLock,
}

impl ClientLock {
    pub(crate) fn acquire(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            _lock: InstanceLock::acquire(path, "Client already running")?,
        })
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
        assert!(err.contains("Lock file unavailable"), "unexpected error: {err}");
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
