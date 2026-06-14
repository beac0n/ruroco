#!/usr/bin/env bash

cargo install xbuild
x run --features with-vendored-openssl --features android-build --device $(x devices | awk '/^adb:/ { print $1 }')
