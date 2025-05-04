#!/usr/bin/env bash

cargo install xbuild
x run --features release-build --features android-build --device $(x devices | awk '/^adb:/ { print $1 }')
