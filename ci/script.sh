#!/usr/bin/env bash

set -eux

# We always want backtraces for everything.
export RUST_BACKTRACE=1

cargo build $PROFILE
cargo test $PROFILE
cargo run --example list_segments

if [[ "$PROFILE" == "--release" ]]; then
    cargo bench
fi
