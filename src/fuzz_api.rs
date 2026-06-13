//! Public entry points for the libFuzzer targets under `fuzz/`. Only compiled with the `fuzzing`
//! feature (which pulls in `with-server`), so these never ship in any real binary.
//!
//! The 93-byte UDP datagram is the only untrusted input surface on the server. This module drives
//! arbitrary bytes through the server-side ingest chain to confirm no malformed input can panic,
//! hang, or read out of bounds:
//!   `DataParser::decode` -> `CryptoHandler::decrypt` -> `ClientData::deserialize`.

use crate::common::client_data::ClientData;
use crate::common::crypto_handler::CryptoHandler;
use crate::common::data_parser::DataParser;
use crate::common::protocol::{KEY_ID_SIZE, MSG_SIZE, PLAINTEXT_SIZE};

/// Feed arbitrary fuzzer bytes through the full server ingest path.
///
/// Inputs shorter than `MSG_SIZE` are zero-padded and longer ones truncated, mirroring that the
/// socket only ever hands the chain an exact `MSG_SIZE` datagram.
pub fn fuzz_server_ingest(data: &[u8]) {
    // A fixed all-zero key. The id won't match anything in a real keystore, but the fuzz path calls
    // `decrypt` directly so the key only has to be structurally valid (8-byte id + 32-byte key).
    let handler = CryptoHandler {
        key: [0u8; 32],
        id: [0u8; KEY_ID_SIZE],
    };

    let mut packet = [0u8; MSG_SIZE];
    let n = data.len().min(MSG_SIZE);
    packet[..n].copy_from_slice(&data[..n]);

    // Path 1: decode -> decrypt. The AEAD tag check rejects essentially all random input, but the
    // slicing and OpenSSL calls must stay panic-free regardless.
    if let Ok((_key_id, ciphertext)) = DataParser::decode(&packet) {
        if let Ok(plaintext) = handler.decrypt(ciphertext) {
            let _ = ClientData::deserialize(plaintext);
        }
    }

    // Path 2: `deserialize` is only reachable post-authentication in production, so the fuzzer could
    // never forge a valid tag to reach it via path 1. Exercise the byte parsing directly on
    // arbitrary plaintext to cover that branch.
    let mut plaintext = [0u8; PLAINTEXT_SIZE];
    let m = data.len().min(PLAINTEXT_SIZE);
    plaintext[..m].copy_from_slice(&data[..m]);
    let _ = ClientData::deserialize(plaintext);
}
