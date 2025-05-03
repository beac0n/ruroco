#!/usr/bin/env bash

cargo build --color=always --release --package ruroco --features release-build --target x86_64-unknown-linux-gnu

# see https://github.com/johnthagen/min-sized-rust
upx --best --lzma target/x86_64-unknown-linux-gnu/release/client_ui
upx --best --lzma target/x86_64-unknown-linux-gnu/release/client
upx --best --lzma target/x86_64-unknown-linux-gnu/release/server
upx --best --lzma target/x86_64-unknown-linux-gnu/release/commander
