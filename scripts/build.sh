#!/usr/bin/env bash

set -ex

cd viewer
cargo clean
wasm-pack build --target web --no-default-features
cp -f pkg/small_world* ../www
