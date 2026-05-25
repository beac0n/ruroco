use anyhow::bail;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct RateLimiter(HashMap<IpAddr, (Instant, u32)>);

impl RateLimiter {
    pub(crate) fn new() -> Self {
        RateLimiter(HashMap::new())
    }

    pub(crate) fn check(&mut self, ip: IpAddr, max: u32) -> anyhow::Result<()> {
        let entry = self.0.entry(ip).or_insert_with(|| (Instant::now(), 0));
        if entry.0.elapsed() >= Duration::from_secs(1) {
            entry.0 = Instant::now();
            entry.1 = 1;
        } else if entry.1 >= max {
            bail!("Rate limit exceeded for {ip}: more than {max} requests per second");
        } else {
            entry.1 += 1;
        }
        Ok(())
    }
}
