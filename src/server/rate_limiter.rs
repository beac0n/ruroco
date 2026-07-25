use anyhow::bail;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) struct RateLimiter {
    per_ip: HashMap<IpAddr, (Instant, u32)>,
    global: (Instant, u32),
    last_sweep: Instant,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        RateLimiter {
            per_ip: HashMap::new(),
            global: (Instant::now(), 0),
            last_sweep: Instant::now(),
        }
    }

    pub(crate) fn check(
        &mut self,
        ip: IpAddr,
        max_per_ip: u32,
        max_global: u32,
    ) -> anyhow::Result<()> {
        self.sweep_if_due();

        // An unknown source only earns a map entry once the global cap has room for it.
        if !self.per_ip.contains_key(&ip) && !Self::admits(&self.global, max_global) {
            bail!("Global rate limit exceeded: more than {max_global} requests per second");
        }

        let per_ip = self.per_ip.entry(ip).or_insert_with(|| (Instant::now(), 0));
        if !Self::bump(per_ip, max_per_ip) {
            bail!("Rate limit exceeded for {ip}: more than {max_per_ip} requests per second");
        }

        if !Self::bump(&mut self.global, max_global) {
            bail!("Global rate limit exceeded: more than {max_global} requests per second");
        }
        Ok(())
    }

    fn admits(entry: &(Instant, u32), max: u32) -> bool {
        entry.0.elapsed() >= WINDOW || entry.1 < max
    }

    fn bump(entry: &mut (Instant, u32), max: u32) -> bool {
        if entry.0.elapsed() >= WINDOW {
            *entry = (Instant::now(), 1);
            true
        } else if entry.1 >= max {
            false
        } else {
            entry.1 += 1;
            true
        }
    }

    fn sweep_if_due(&mut self) {
        if self.last_sweep.elapsed() < WINDOW {
            return;
        }
        self.last_sweep = Instant::now();
        self.per_ip.retain(|_, (since, _)| since.elapsed() < WINDOW);
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
            limiter.check(ip(n), 2, u32::MAX).unwrap();
        }
        assert_eq!(limiter.per_ip.len(), 1000);

        sleep(Duration::from_secs(1));

        // A single fresh check sweeps every elapsed entry, leaving only itself.
        limiter.check(ip(10_000), 2, u32::MAX).unwrap();
        assert_eq!(limiter.per_ip.len(), 1);
    }

    #[test]
    fn test_spoofed_flood_cannot_grow_the_map_past_the_global_cap() {
        let mut limiter = RateLimiter::new();

        // 10 packets exhaust the global budget; the other 990 come from unique spoofed sources
        // and are rejected, so none of them may leave an entry behind.
        for n in 0..1000 {
            let _ = limiter.check(ip(n), 2, 10);
        }

        assert_eq!(limiter.per_ip.len(), 10);
    }

    #[test]
    fn test_rejected_new_source_does_not_consume_global_budget() {
        let mut limiter = RateLimiter::new();
        assert!(limiter.check(ip(1), u32::MAX, 1).is_ok());

        assert!(limiter.check(ip(2), u32::MAX, 1).is_err());
        assert!(limiter.check(ip(3), u32::MAX, 1).is_err());

        // the known peer still has its own budget: the rejections above changed nothing
        assert!(limiter.check(ip(1), u32::MAX, 1).is_err());
        assert_eq!(limiter.per_ip.len(), 1);
    }

    #[test]
    fn test_stale_entry_left_by_a_throttled_sweep_still_admits_its_peer() {
        let mut limiter = RateLimiter::new();
        assert!(limiter.check(ip(1), 1, u32::MAX).is_ok());
        assert!(limiter.check(ip(1), 1, u32::MAX).is_err());

        sleep(Duration::from_millis(1050));

        // The sweep runs at most once per window, so the entry may well still be in the map. Its
        // window has elapsed either way, so `bump` resets it and the peer is admitted again.
        assert!(limiter.check(ip(1), 1, u32::MAX).is_ok());
    }

    #[test]
    fn test_rate_limit_enforced_within_window() {
        let mut limiter = RateLimiter::new();
        assert!(limiter.check(ip(1), 2, u32::MAX).is_ok());
        assert!(limiter.check(ip(1), 2, u32::MAX).is_ok());
        assert!(limiter.check(ip(1), 2, u32::MAX).is_err());
    }

    #[test]
    fn test_global_limit_blocks_spoofed_ip_flood() {
        // Each request comes from a distinct source IP, so the per-IP limit (generous here) never
        // trips; only the global cap can stop the flood.
        let mut limiter = RateLimiter::new();
        assert!(limiter.check(ip(1), u32::MAX, 2).is_ok());
        assert!(limiter.check(ip(2), u32::MAX, 2).is_ok());
        let err = limiter.check(ip(3), u32::MAX, 2).unwrap_err().to_string();
        assert!(err.contains("Global rate limit exceeded"), "unexpected error: {err}");
    }

    #[test]
    fn test_per_ip_rejection_does_not_consume_global_budget() {
        // ip(1) burns its per-IP budget; those rejected packets must not count toward the global
        // cap, so a different IP still gets its full global allowance afterwards.
        let mut limiter = RateLimiter::new();
        assert!(limiter.check(ip(1), 1, 5).is_ok()); // ip(1): 1 ok (global=1)
        assert!(limiter.check(ip(1), 1, 5).is_err()); // ip(1): per-IP rejected, global untouched
        assert!(limiter.check(ip(1), 1, 5).is_err());
        // global should still be at 1, so four more distinct IPs fit under the cap of 5
        for n in 2..=5 {
            assert!(limiter.check(ip(n), 1, 5).is_ok(), "global budget wrongly consumed at {n}");
        }
    }
}
