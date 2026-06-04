# Blocklist and Rate Limiter

These two modules implement the server's two independent defenses against abuse: the **blocklist**
(`blocklist.rs`) provides durable replay protection, and the **rate limiter** (`rate_limiter.rs`)
provides in-memory throttling. They serve different purposes and must not be confused: the
blocklist is security (it rejects replayed and stale packets across restarts), the rate limiter is
load protection (it caps requests per second and forgets everything on restart).

## `blocklist.rs`

### Responsibilities

Tracks the highest counter accepted per `key_id` and persists it to disk as MessagePack so replay
protection survives restarts. Each key has its own counter floor.

### Type

```rust
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Blocklist {
    map: HashMap<[u8; KEY_ID_SIZE], u128>, // KEY_ID_SIZE == 8
    path: PathBuf,
}
```

The `map` is "key id -> most recent counter accepted". The counter is a u128 nanosecond timestamp,
not a sequential number, so the stored value jumps forward by large amounts and gaps are expected.

### Persistence (MessagePack)

```rust
pub fn create(config_dir: &Path) -> anyhow::Result<Blocklist>;
pub fn get_blocklist_path(config_dir: &Path) -> PathBuf; // config_dir/blocklist.msgpck
pub(crate) fn save(&self) -> anyhow::Result<()>;
```

- `create` reads `config_dir/blocklist.msgpck` if it exists and deserializes it with `rmp_serde`
  (a corrupted file is a hard error: `"Could not create blocklist from vec"`), otherwise starts
  with an empty map. It then immediately `save()`s, so the file always exists after `create`.
- `save` serializes the whole struct with `rmp_serde::to_vec` and writes it through
  `write_atomic` (temp file, fsync, rename) so a crash mid-write cannot corrupt the file.

### The replay check (`>=` semantics)

```rust
pub(crate) fn is_counter_replayed(&self, key_id: [u8; KEY_ID_SIZE], value: u128) -> bool {
    match self.map.get(&key_id) {
        Some(v) => v >= &value,
        None => true,
    }
}
```

This returns `true` (replayed, reject) when:

- the stored counter is **greater than or equal to** the incoming `value`. Equal counts as a
  replay: the stored value records the most recent counter accepted, so an identical counter is a
  retransmit, capture, or adversarial replay and must be rejected. Do not relax this to `>`.
- **or the key id is unknown** (`None`). An entry that has never been seeded is treated as blocked.
  In normal operation this cannot happen for a configured key because every key is seeded at
  startup (below), but it makes the default safe.

### Startup seeding to `now_nanos`

In `Server::create`:

```rust
let floor = now_nanos()?;
for key_id in crypto_handlers.keys() {
    blocklist.seed_if_absent(*key_id, floor);
}
blocklist.save()?;
```

```rust
pub(crate) fn seed_if_absent(&mut self, key_id: [u8; KEY_ID_SIZE], floor: u128) {
    self.map.entry(key_id).or_insert(floor);
}
```

Every loaded key gets its counter floor seeded to the current nanosecond timestamp **only if it is
absent**. An existing entry from a previous run is never overwritten. The effect: after a (re)start,
any packet whose counter is older than the moment the process came up is rejected, even one that was
never seen before. `seed_if_absent` uses `entry().or_insert()` so a higher persisted value wins over
the startup floor.

### Other methods

```rust
pub(crate) fn get_counter(&self, key_id: [u8; KEY_ID_SIZE]) -> Option<&u128>;
pub fn get(&self) -> &HashMap<[u8; KEY_ID_SIZE], u128>;
pub(crate) fn add(&mut self, key_id: [u8; KEY_ID_SIZE], entry: u128);
```

`add` unconditionally inserts (overwrites) the counter for a key; the handler only calls it after
the replay check has passed, so it always moves the floor upward.

### Gotchas

- Equal counter = replay. This is intentional and load-bearing for security.
- An unknown key id is treated as blocked, not allowed.
- The whole map is rewritten on every accepted packet (`add` then `save`). This is fine for the
  expected low request volume and gives crash-safe atomic persistence.

## `rate_limiter.rs`

### Responsibilities

Caps the number of accepted requests per source IP within a rolling ~1-second window. This is
throttling to limit decrypt work and command floods; it is **not** replay defense and provides no
guarantees across restarts.

### Type and methods

```rust
#[derive(Debug)]
pub(crate) struct RateLimiter(HashMap<IpAddr, (Instant, u32)>);

impl RateLimiter {
    pub(crate) fn new() -> Self;
    pub(crate) fn check(&mut self, ip: IpAddr, max: u32) -> anyhow::Result<()>;
}
```

Each IP maps to a `(window_start: Instant, count: u32)` pair.

### The window logic

```rust
let entry = self.0.entry(ip).or_insert_with(|| (Instant::now(), 0));
if entry.0.elapsed() >= Duration::from_secs(1) {
    entry.0 = Instant::now(); // window expired, reset
    entry.1 = 1;
} else if entry.1 >= max {
    bail!("Rate limit exceeded for {ip}: more than {max} requests per second");
} else {
    entry.1 += 1;
}
Ok(())
```

- First request from an IP creates an entry and counts as 1.
- If at least 1 second has elapsed since the window started, the window resets and the count goes
  back to 1.
- Within the window, once `count >= max` the request is rejected with `"Rate limit exceeded"`.
- Otherwise the count is incremented and the request passes.

### Default and wiring

The limit comes from `ConfigServer::max_requests_per_second`, whose default is **2** (see
`default_max_requests_per_second`). The server calls it from `check_rate_limit`:

```rust
self.rate_limiter.check(src_ip, self.config.max_requests_per_second)
```

This runs **before** decryption in the receive loop, so a flood of garbage packets from one IP is
throttled before the relatively expensive AES-256-GCM decrypt.

### Gotchas

- In-memory only: the `HashMap` is rebuilt empty on every process start. Restarting the server
  clears all rate-limit state.
- It is a sliding window keyed on the first request's `Instant`, not a fixed calendar second, so two
  bursts straddling a window boundary are each limited independently.
- It throttles, it does not authenticate or detect replays. Replay defense is entirely the
  blocklist's job.
- The map grows one entry per distinct source IP and is never pruned within the process lifetime.
