#!/usr/bin/env bash

cargo install xbuild
x build --features release-build --features android-build --release --platform android --arch arm64 --format apk --verbose
