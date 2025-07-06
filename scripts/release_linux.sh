#!/usr/bin/env bash

cargo build --color=always --release --package ruroco --features release-build --target x86_64-unknown-linux-gnu

# see https://github.com/johnthagen/min-sized-rust
upx --best --lzma target/x86_64-unknown-linux-gnu/release/client_ui -o target/x86_64-unknown-linux-gnu/release/client_ui_min
mv target/x86_64-unknown-linux-gnu/release/client_ui_min target/x86_64-unknown-linux-gnu/release/client_ui

upx --best --lzma target/x86_64-unknown-linux-gnu/release/client -o target/x86_64-unknown-linux-gnu/release/client_min
mv target/x86_64-unknown-linux-gnu/release/client target/x86_64-unknown-linux-gnu/release/client_min

upx --best --lzma target/x86_64-unknown-linux-gnu/release/server -o target/x86_64-unknown-linux-gnu/release/server_min
mv target/x86_64-unknown-linux-gnu/release/server_min target/x86_64-unknown-linux-gnu/release/server

upx --best --lzma target/x86_64-unknown-linux-gnu/release/commander -o target/x86_64-unknown-linux-gnu/release/commander_min
mv target/x86_64-unknown-linux-gnu/release/commander target/x86_64-unknown-linux-gnu/release/commander_min
