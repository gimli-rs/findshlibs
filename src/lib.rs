//! # `findshlibs`
//!
//! Find the set of shared libraries currently loaded in this process with a
//! cross platform API.

#![deny(missing_docs)]

#[cfg(target_os = "linux")]
pub mod linux;

/// The [`SharedLibrary` trait](./trait.SharedLibrary.html) implementation for
/// the target operating system.
#[cfg(target_os = "linux")]
pub type TargetSharedLibrary<'a> = linux::SharedLibrary<'a>;

mod shared_lib;

pub use shared_lib::SharedLibrary;
pub use shared_lib::IterationControl;
