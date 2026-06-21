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
        if counter.read().is_err() {
            counter.count = initial;
            counter.write()?;
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

    fn read(&mut self) -> anyhow::Result<()> {
        let mut buf = [0u8; 16];
        File::open(&self.path)
            .with_context(|| format!("Could not open counter file {:?}", &self.path))?
            .read_exact(&mut buf)
            .with_context(|| format!("Could not read counter file {:?}", &self.path))?;

        self.count = u128::from_be_bytes(buf);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_counter(initial: u128) -> Counter {
        let dir = tempfile::tempdir().unwrap();
        Counter::create_and_init(dir.keep().join("counter"), initial).unwrap()
    }

    #[test]
    fn test_inc_normal() {
        let mut c = make_counter(0);
        c.inc().unwrap();
        assert_eq!(c.count(), 1);
    }

    #[test]
    fn test_inc_overflow_returns_error() {
        let mut c = make_counter(u128::MAX);
        assert!(c.inc().is_err());
        let err = c.inc().unwrap_err().to_string();
        assert!(err.contains("counter overflow"), "unexpected error: {err}");
    }

    #[test]
    fn test_inc_near_max_does_not_overflow() {
        let mut c = make_counter(u128::MAX - 1);
        c.inc().unwrap();
        assert_eq!(c.count(), u128::MAX);
    }

    #[test]
    fn test_reseed_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep().join("counter");
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
        let path = dir.keep().join("counter");
        let mut c1 = Counter::create_and_init(path.clone(), 10).unwrap();
        c1.inc().unwrap(); // persists count=11
        drop(c1);

        // Re-init on existing file should read 11, not reset to 99
        let c2 = Counter::create_and_init(path, 99).unwrap();
        assert_eq!(c2.count(), 11);
    }
}
