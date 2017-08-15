#!/usr/bin/env bash

# We always want backtraces for everything.
export RUST_BACKTRACE=1

case "$TRAVIS_OS_NAME" in
    "osx")
        brew update
        brew install llvm@3.9
        export LIBCLANG_PATH=$(find /usr/local/Cellar/llvm -type f -name libclang.dylib | head -n 1)
        export LIBCLANG_PATH=$(dirname $LIBCLANG_PATH)
        ;;

    "linux")
        export LIBCLANG_PATH=/usr/lib/llvm-3.9/lib
        ;;

    *)
        echo "Error: unknown \$TRAVIS_OS_NAME: $TRAVIS_OS_NAME"
        exit 1
        ;;
esac
