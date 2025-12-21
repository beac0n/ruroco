use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct Counter {
    path: PathBuf,
    count: u128,
}

impl Counter {
    pub fn create(path: PathBuf, initial: u128) -> Result<Self, String> {
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

    pub fn dec(&mut self) -> Result<(), String> {
        self.count = self.count.saturating_sub(1);
        self.write()?;
        Ok(())
    }

    pub(crate) fn inc(&mut self) -> Result<(), String> {
        self.count = self.count.saturating_add(1);
        self.write()?;
        Ok(())
    }

    fn write(&self) -> Result<(), String> {
        File::create(&self.path)
            .map_err(|e| format!("Could not create counter file {:?}: {e}", self.path))?
            .write_all(&self.count.to_be_bytes())
            .map_err(|e| format!("Could not write counter file {:?}: {e}", self.path))
    }

    fn read(&mut self) -> Result<(), String> {
        let mut buf = [0u8; 16];
        File::open(&self.path)
            .map_err(|e| format!("Could not open counter file {:?}: {e}", &self.path))?
            .read_exact(&mut buf)
            .map_err(|e| format!("Could not read counter file {:?}: {e}", &self.path))?;

        self.count = u128::from_be_bytes(buf);
        Ok(())
    }
}
