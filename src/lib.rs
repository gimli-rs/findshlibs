//! # `findshlibs`
//!
//! Find the set of shared libraries currently loaded in this process with a
//! cross platform API.
//!
//! ## Example
//!
//! Here is an example program that prints out each shared library that is
//! loaded in the process and the addresses where each of its segments are
//! mapped into memory.
//!
//! ```
//! extern crate findshlibs;
//! use findshlibs::{Segment, SharedLibrary, TargetSharedLibrary};
//!
//! fn main() {
//!     TargetSharedLibrary::each(|shlib| {
//!         println!("{}", shlib.name().to_string_lossy());
//!
//!         for seg in shlib.segments() {
//!             println!("    0x{:x}: segment {}",
//!                      seg.actual_virtual_memory_address(shlib),
//!                      seg.name().to_string_lossy());
//!         }
//!     });
//! }
//! ```
//!
//! ## Addresses
//!
//! Shared libraries' addresses can be confusing. They are loaded somewhere in
//! physical memory, but we generally don't care about their addresses in
//! physical memory, because only the OS can see that address and we can only
//! access them through their virtual memory address. But even "virtual memory
//! address" is ambiguous because it isn't clear whether this is the address
//! before or after the loader maps the object into memory and performs
//! relocation.
//!
//! To clarify between these different kinds of addresses, we borrow some
//! terminology from [LUL][]:
//!
//! > * SVMA ("Stated Virtual Memory Address"): this is an address of a
//! >   symbol (etc) as it is stated in the symbol table, or other
//! >   metadata, of an object.  Such values are typically small and
//! >   start from zero or thereabouts, unless the object has been
//! >   prelinked.
//! >
//! > * AVMA ("Actual Virtual Memory Address"): this is the address of a
//! >   symbol (etc) in a running process, that is, once the associated
//! >   object has been mapped into a process.  Such values are typically
//! >   much larger than SVMAs, since objects can get mapped arbitrarily
//! >   far along the address space.
//! >
//! > * "Bias": the difference between AVMA and SVMA for a given symbol
//! >   (specifically, AVMA - SVMA).  The bias is always an integral
//! >   number of pages.  Once we know the bias for a given object's
//! >   text section (for example), we can compute the AVMAs of all of
//! >   its text symbols by adding the bias to their SVMAs.
//!
//! [LUL]: http://searchfox.org/mozilla-central/rev/13148faaa91a1c823a7d68563d9995480e714979/tools/profiler/lul/LulMain.h#17-51

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

pub use shared_lib::Segment;
pub use shared_lib::SharedLibrary;
pub use shared_lib::IterationControl;
