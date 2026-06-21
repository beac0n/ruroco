use anyhow::bail;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct RateLimiter {
    per_ip: HashMap<IpAddr, (Instant, u32)>,
    global: (Instant, u32),
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        RateLimiter {
            per_ip: HashMap::new(),
            global: (Instant::now(), 0),
        }
    }

    /// Enforce a per-source-IP limit and a global (all-sources) limit within a ~1s window.
    ///
    /// The per-IP cap throttles a single chatty or abusive peer. The global cap bounds total work
    /// (chiefly decrypt attempts) under a flood of spoofed source IPs, which the per-IP map alone
    /// cannot stop: every spoofed address looks like a brand-new peer and sails past its own fresh
    /// per-IP counter. The per-IP check runs first so a rejected packet never consumes global
    /// budget meant for legitimate peers.
    pub(crate) fn check(
        &mut self,
        ip: IpAddr,
        max_per_ip: u32,
        max_global: u32,
    ) -> anyhow::Result<()> {
        self.drop_old_entries();

        let per_ip = self.per_ip.entry(ip).or_insert_with(|| (Instant::now(), 0));
        if !Self::bump(per_ip, max_per_ip) {
            bail!("Rate limit exceeded for {ip}: more than {max_per_ip} requests per second");
        }

        if !Self::bump(&mut self.global, max_global) {
            bail!("Global rate limit exceeded: more than {max_global} requests per second");
        }
        Ok(())
    }

    /// Advance a `(window_start, count)` slot for the current ~1s window. Returns `false` when the
    /// slot has already reached `max` this window (caller should reject), `true` otherwise.
    fn bump(entry: &mut (Instant, u32), max: u32) -> bool {
        if entry.0.elapsed() >= Duration::from_secs(1) {
            *entry = (Instant::now(), 1);
            true
        } else if entry.1 >= max {
            false
        } else {
            entry.1 += 1;
            true
        }
    }

    /// Lazy sweep: drop entries whose window has elapsed so the map can never grow unbounded under
    /// a flood of (spoofable) unique source IPs.
    fn drop_old_entries(&mut self) {
        self.per_ip.retain(|_, (since, _)| since.elapsed() < Duration::from_secs(1));
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
