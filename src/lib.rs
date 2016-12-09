//! # `findshlibs`
//!
//! Find the set of shared libraries currently loaded in this process with a
//! cross platform API.

#![deny(missing_docs)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate lazy_static;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

cfg_if!(
    if #[cfg(target_os = "linux")] {
/// The [`SharedLibrary` trait](./trait.SharedLibrary.html) implementation for
/// the target operating system.
        pub type TargetSharedLibrary<'a> = linux::SharedLibrary<'a>;
    } else if #[cfg(target_os = "macos")] {
/// The [`SharedLibrary` trait](./trait.SharedLibrary.html) implementation for
/// the target operating system.
        pub type TargetSharedLibrary<'a> = macos::SharedLibrary<'a>;
    } else {
// No implementation for this platform :(
    }
);


mod shared_lib;

pub use shared_lib::SharedLibrary;
pub use shared_lib::IterationControl;
