#!/usr/bin/env bash

# Usage:
#
#     ./regen_linux_bindings path/to/bindgen/executable
#
# Regenerate the linux bindings for dl_iterate_phdr and friends.

set -xe

cd $(dirname $0)

BINDGEN=$1

$BINDGEN \
    --raw-line '#![allow(non_snake_case)]' \
    --raw-line '#![allow(non_camel_case_types)]' \
    --whitelist-function dl_iterate_phdr \
    ./src/linux/bindings.h \
    > ./src/linux/bindings.rs
