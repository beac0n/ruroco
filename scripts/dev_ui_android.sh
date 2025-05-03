#!/usr/bin/env bash

x run --features release-build --features android-build --device $(x devices | awk '/^adb:/ { print $1 }')
