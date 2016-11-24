#!/usr/bin/env bash

# Usage:
#
#     ./regen_macos_bindings path/to/bindgen/executable
#
# Regenerate the macos bindings for _dyld_*.

set -xe

cd $(dirname $0)

BINDGEN=$1

export DYLD_LIBRARY_PATH=~/src/llvm/obj/lib/
export LIBCLANG_PATH=~/src/llvm/obj/lib/

$BINDGEN \
    --raw-line '#![allow(non_snake_case)]' \
    --raw-line '#![allow(non_camel_case_types)]' \
    --whitelist-function '_dyld_.*' \
    ./src/macos/bindings.h \
    > ./src/macos/bindings.rs
