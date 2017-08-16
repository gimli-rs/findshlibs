# `findshlibs`

[![](http://meritbadge.herokuapp.com/findshlibs) ![](https://img.shields.io/crates/d/findshlibs.png)](https://crates.io/crates/findshlibs) [![Build Status](https://travis-ci.org/fitzgen/findshlibs.png?branch=master)](https://travis-ci.org/fitzgen/findshlibs) [![Coverage Status](https://coveralls.io/repos/github/fitzgen/findshlibs/badge.svg?branch=master)](https://coveralls.io/github/fitzgen/findshlibs?branch=master)

Find the shared libraries loaded in the current process with a cross platform
API.

## Documentation

[📚 Documentation on docs.rs 📚](https://docs.rs/findshlibs)

## Example

Here is an example program that prints out each shared library that is
loaded in the process and the addresses where each of its segments are
mapped into memory.

```rust
extern crate findshlibs;
use findshlibs::{Segment, SharedLibrary, TargetSharedLibrary};

fn main() {
    TargetSharedLibrary::each(|shlib| {
        println!("{}", shlib.name().to_string_lossy());

        for seg in shlib.segments() {
            println!("    {}: segment {}",
                     seg.actual_virtual_memory_address(shlib),
                     seg.name().to_string_lossy());
        }
    });
}
```

## Supported OSes

These are the OSes that `findshlibs` currently supports:

* Linux
* macOS

Is your OS missing here? Send us a pull request!
