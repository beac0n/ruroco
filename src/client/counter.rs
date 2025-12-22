use anyhow::Context;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct Counter {
    path: PathBuf,
    count: u128,
}

impl Counter {
    pub fn create(path: PathBuf, initial: u128) -> anyhow::Result<Self> {
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

    pub fn dec(&mut self) -> anyhow::Result<()> {
        self.count = self.count.saturating_sub(1);
        self.write()?;
        Ok(())
    }

    pub(crate) fn inc(&mut self) -> anyhow::Result<()> {
        self.count = self.count.saturating_add(1);
        self.write()?;
        Ok(())
    }

    fn write(&self) -> anyhow::Result<()> {
        File::create(&self.path)
            .with_context(|| format!("Could not create counter file {:?}", self.path))?
            .write_all(&self.count.to_be_bytes())
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
