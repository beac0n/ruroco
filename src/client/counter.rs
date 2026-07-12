use crate::common::fs::write_atomic;
use anyhow::Context;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Counter {
    path: PathBuf,
    count: u128,
}

impl Counter {
    pub fn create_and_init(path: PathBuf, initial: u128) -> anyhow::Result<Self> {
        let mut counter = Self { path, count: 0 };
        match counter.read() {
            Ok(()) => {}
            // No counter file yet (first run): seed it.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                counter.count = initial;
                counter.write()?;
            }
            // Any other error (corrupt/truncated file, permission problem, ...) must not be
            // papered over by silently reseeding: that would mask the underlying problem and,
            // if the file held a legitimately future-dated counter, move it backwards - causing
            // every subsequent send to be rejected as a replay by the server. Surface it instead;
            // `ruroco-client reseed` is the explicit, deliberate way to reset the counter.
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Could not read counter file {:?}", &counter.path));
            }
        }
        Ok(counter)
    }

    pub(crate) fn count(&self) -> u128 {
        self.count
    }

    pub(crate) fn inc(&mut self) -> anyhow::Result<()> {
        self.count = self.count.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!(
                "counter overflow: value has reached u128::MAX ({}) and cannot be incremented",
                u128::MAX
            )
        })?;
        self.write()?;
        Ok(())
    }

    pub fn reseed(path: PathBuf, value: u128) -> anyhow::Result<()> {
        Self { path, count: value }.write()
    }

    fn write(&self) -> anyhow::Result<()> {
        // Atomic temp-file + fsync + rename so a crash mid-write can never leave a torn or
        // truncated counter on disk (which would weaken replay protection on the next send).
        write_atomic(&self.path, &self.count.to_be_bytes())
            .with_context(|| format!("Could not write counter file {:?}", self.path))
    }

    fn read(&mut self) -> std::io::Result<()> {
        let mut buf = [0u8; 16];
        File::open(&self.path)?.read_exact(&mut buf)?;
        self.count = u128::from_be_bytes(buf);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_counter(initial: u128) -> (TempDir, Counter) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("counter");
        let counter = Counter::create_and_init(path, initial).unwrap();
        (dir, counter)
    }

    #[test]
    fn test_inc_normal() {
        let (_dir, mut c) = make_counter(0);
        c.inc().unwrap();
        assert_eq!(c.count(), 1);
    }

    #[test]
    fn test_inc_overflow_returns_error() {
        let (_dir, mut c) = make_counter(u128::MAX);
        assert!(c.inc().is_err());
        let err = c.inc().unwrap_err().to_string();
        assert!(err.contains("counter overflow"), "unexpected error: {err}");
    }

    #[test]
    fn test_inc_near_max_does_not_overflow() {
        let (_dir, mut c) = make_counter(u128::MAX - 1);
        c.inc().unwrap();
        assert_eq!(c.count(), u128::MAX);
    }

    #[test]
    fn test_reseed_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("counter");
        let mut c = Counter::create_and_init(path.clone(), 100).unwrap();
        c.inc().unwrap(); // persists 101
        drop(c);

        Counter::reseed(path.clone(), 9999).unwrap();
        let c2 = Counter::create_and_init(path, 0).unwrap();
        assert_eq!(c2.count(), 9999);
    }

    #[test]
    fn test_write_to_invalid_dir_returns_error() {
        let path = PathBuf::from("/tmp/no_such_dir_ruroco_xyz/counter");
        let result = Counter::create_and_init(path, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not write counter file"));
    }

    #[test]
    fn test_create_and_init_reads_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("counter");
        let mut c1 = Counter::create_and_init(path.clone(), 10).unwrap();
        c1.inc().unwrap(); // persists count=11
        drop(c1);

        // Re-init on existing file should read 11, not reset to 99
        let c2 = Counter::create_and_init(path, 99).unwrap();
        assert_eq!(c2.count(), 11);
    }

    #[test]
    fn test_create_and_init_does_not_silently_reseed_on_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("counter");
        // Not 16 bytes, so read_exact fails - this must surface as an error, not a silent reseed.
        std::fs::write(&path, b"short").unwrap();

        let result = Counter::create_and_init(path.clone(), 42);

        assert!(result.unwrap_err().to_string().contains("Could not read counter file"));
        // The corrupt file must be left untouched, not silently overwritten with `initial`.
        assert_eq!(std::fs::read(&path).unwrap(), b"short");
    }

    #[cfg(unix)]
    #[test]
    fn test_create_and_init_does_not_silently_reseed_on_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        if nix::unistd::Uid::effective().is_root() {
            // root ignores file permission bits, so the read below would succeed anyway.
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("counter");
        std::fs::write(&path, 999u128.to_be_bytes()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = Counter::create_and_init(path.clone(), 42);

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(result.unwrap_err().to_string().contains("Could not read counter file"));
        assert_eq!(u128::from_be_bytes(std::fs::read(&path).unwrap().try_into().unwrap()), 999);
    }
}
