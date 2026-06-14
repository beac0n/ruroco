#!/usr/bin/env bash

cargo install xbuild
x build --features with-vendored-openssl --features android-build --release --platform android --arch arm64 --format apk --verbose
