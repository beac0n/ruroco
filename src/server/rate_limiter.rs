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
        self.drop_old_entries();
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

    /// Lazy sweep: drop entries whose window has elapsed so the map can never grow unbounded under
    /// a flood of (spoofable) unique source IPs.
    fn drop_old_entries(&mut self) {
        self.0.retain(|_, (since, _)| since.elapsed() < Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::thread::sleep;

    fn ip(n: u32) -> IpAddr {
        IpAddr::V4(Ipv4Addr::from(n))
    }

    #[test]
    fn test_evicts_stale_entries_under_unique_ip_flood() {
        let mut limiter = RateLimiter::new();
        for n in 0..1000 {
            limiter.check(ip(n), 2).unwrap();
        }
        assert_eq!(limiter.0.len(), 1000);

        sleep(Duration::from_secs(1));

        // A single fresh check sweeps every elapsed entry, leaving only itself.
        limiter.check(ip(10_000), 2).unwrap();
        assert_eq!(limiter.0.len(), 1);
    }

    #[test]
    fn test_rate_limit_enforced_within_window() {
        let mut limiter = RateLimiter::new();
        assert!(limiter.check(ip(1), 2).is_ok());
        assert!(limiter.check(ip(1), 2).is_ok());
        assert!(limiter.check(ip(1), 2).is_err());
    }
}
