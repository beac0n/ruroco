#![no_main]

//! Fuzzes the untrusted server-side packet path: decode -> decrypt -> deserialize.
//! Run with `cargo +nightly fuzz run parse_path` (see `make fuzz`).

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    ruroco::fuzz_api::fuzz_server_ingest(data);
});
