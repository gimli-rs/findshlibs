//! # `findshlibs`
//!
//! Find the set of shared libraries (and current executable) currently mapped
//! into this process with a cross platform API.
//!
//! The API entry point is the `TargetSharedLibrary` type and its
//! `SharedLibrary::each` trait method implementation.
//!
//! ## Example
//!
//! Here is an example program that prints out each shared library that is
//! loaded in the process, and the addresses where each of its segments are
//! mapped into memory.
//!
//! ```
//! extern crate findshlibs;
//! use findshlibs::{NamedMemoryRange, SharedLibrary, TargetSharedLibrary};
//!
//! fn main() {
//!     TargetSharedLibrary::each(|shlib| {
//!         println!("{}", shlib.name().to_string_lossy());
//!
//!         for segment in shlib.segments() {
//!             println!(
//!                 "    {}: segment {}",
//!                 segment.actual_virtual_memory_address(shlib),
//!                 segment.name().to_string_lossy()
//!             );
//!         }
//!     });
//! }
//! ```
//!
//! Here is an example program that finds the addresses where the `.eh_frame_hdr`
//! exception handling sections are mapped into memory:
//!
//! ```
//! extern crate findshlibs;
//! use findshlibs::{NamedMemoryRange, SharedLibrary, TargetSharedLibrary};
//!
//! fn main() {
//!     TargetSharedLibrary::each(|shlib| {
//!         println!("{}", shlib.name().to_string_lossy());
//!
//!         if let Some(eh_frame_hdr) = shlib.eh_frame_hdr() {
//!             println!(
//!                 "    .eh_frame_hdr @ {}",
//!                 eh_frame_hdr.actual_virtual_memory_address(shlib),
//!             );
//!         } else {
//!             println!("    (no .eh_frame_hdr)");
//!         }
//!     });
//! }
//! ```
//!
//! ## Supported OSes
//!
//! These are the OSes that `findshlibs` currently supports:
//!
//! * Linux
//! * macOS
//!
//! Is your OS missing here? Send us a pull request!
//!
//! ## Addresses
//!
//! Shared libraries' addresses can be confusing. They are loaded somewhere in
//! physical memory, but we generally don't care about physical memory
//! addresses, because only the OS can see that address and in userspace we can
//! only access memory through its virtual memory address. But even "virtual
//! memory address" is ambiguous because it isn't clear whether this is the
//! address before or after the loader maps the shared library into memory and
//! performs relocation.
//!
//! To clarify between these different kinds of addresses, we borrow some
//! terminology from [LUL][]:
//!
//! > * **SVMA** ("Stated Virtual Memory Address"): this is an address of a
//! >   symbol (etc) as it is stated in the symbol table, or other
//! >   metadata, of an object.  Such values are typically small and
//! >   start from zero or thereabouts, unless the object has been
//! >   prelinked.
//! >
//! > * **AVMA** ("Actual Virtual Memory Address"): this is the address of a
//! >   symbol (etc) in a running process, that is, once the associated
//! >   object has been mapped into a process.  Such values are typically
//! >   much larger than SVMAs, since objects can get mapped arbitrarily
//! >   far along the address space.
//! >
//! > * **"Bias"**: the difference between AVMA and SVMA for a given symbol
//! >   (specifically, AVMA - SVMA).  The bias is always an integral
//! >   number of pages.  Once we know the bias for a given object's
//! >   text section (for example), we can compute the AVMAs of all of
//! >   its text symbols by adding the bias to their SVMAs.
//!
//! [LUL]: http://searchfox.org/mozilla-central/rev/13148faaa91a1c823a7d68563d9995480e714979/tools/profiler/lul/LulMain.h#17-51
#![deny(missing_docs)]

#[macro_use]
extern crate cfg_if;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate lazy_static;

use std::ffi::CStr;
use std::fmt::{self, Debug};
use std::ptr;

cfg_if!(
    if #[cfg(target_os = "linux")] {

        pub mod linux;

        /// The [`SharedLibrary` trait](./trait.SharedLibrary.html)
        /// implementation for the target operating system.
        pub type TargetSharedLibrary<'a> = linux::SharedLibrary<'a>;

    } else if #[cfg(target_os = "macos")] {

        pub mod macos;

        /// The [`SharedLibrary` trait](./trait.SharedLibrary.html)
        /// implementation for the target operating system.
        pub type TargetSharedLibrary<'a> = macos::SharedLibrary<'a>;

    } else {

        // No implementation for this platform :(

    }
);

macro_rules! simple_newtypes {
    (
        $(
            $(#[$attr:meta])*
            type $name:ident = $oldty:ty
            where
                default = $default:expr ,
                display = $format:expr ;
        )*
    ) => {
        $(
            $(#[$attr])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(pub $oldty);

            impl Default for $name {
                #[inline]
                fn default() -> Self {
                    $name( $default )
                }
            }

            impl From<$oldty> for $name {
                fn from(x: $oldty) -> $name {
                    $name(x)
                }
            }

            impl From<$name> for $oldty {
                fn from($name(x): $name) -> $oldty {
                    x
                }
            }

            impl fmt::Display for $name {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, $format, self.0)
                }
            }
        )*
    }
}

simple_newtypes! {
    /// Stated virtual memory address.
    ///
    /// See the module documentation for details.
    type Svma = *const u8
    where
        default = ptr::null(),
        display = "{:#0p}";

    /// Actual virtual memory address.
    ///
    /// See the module documentation for details.
    type Avma = *const u8
    where
        default = ptr::null(),
        display = "{:#0p}";

    /// Virtual memory bias.
    ///
    /// See the module documentation for details.
    type Bias = isize
    where
        default = 0,
        display = "{:#x}";
}

/// A named memory range.
///
/// This trait encapsulates the operations common to both segments and sections.
pub trait NamedMemoryRange<Shlib: SharedLibrary>: Sized + Debug {
    /// Get this memory range or section's name.
    fn name(&self) -> &CStr;

    /// Get this memory range's stated virtual address of this memory range.
    ///
    /// This is the virtual memory address without the bias applied. See the
    /// module documentation for details.
    fn stated_virtual_memory_address(&self) -> Svma;

    /// Get the length of this memory range in memory (in bytes).
    fn len(&self) -> usize;

    // Provided methods.

    /// Get this section's actual virtual memory address.
    ///
    /// This is the virtual memory address with the bias applied. See the module
    /// documentation for details.
    #[inline]
    fn actual_virtual_memory_address(&self, shlib: &Shlib) -> Avma {
        let svma = self.stated_virtual_memory_address();
        let bias = shlib.virtual_memory_bias();
        Avma(unsafe { svma.0.offset(bias.0) })
    }

    /// Does this section contain the given address?
    #[inline]
    fn contains_svma(&self, address: Svma) -> bool {
        let start = self.stated_virtual_memory_address().0 as usize;
        let end = start + self.len();
        let address = address.0 as usize;
        start <= address && address < end
    }

    /// Does this section contain the given address?
    #[inline]
    fn contains_avma(&self, shlib: &Shlib, address: Avma) -> bool {
        let start = self.actual_virtual_memory_address(shlib).0 as usize;
        let end = start + self.len();
        let address = address.0 as usize;
        start <= address && address < end
    }
}

/// An `.eh_frame_hdr` section within a mapped segment.
pub trait EhFrameHdr: NamedMemoryRange<<Self as EhFrameHdr>::SharedLibrary> {
    /// The associated segment type for this `.eh_frame_hdr` section.
    type Segment: Segment<EhFrameHdr = Self>;

    /// The associated shared library type for this `.eh_frame_hdr` section.
    type SharedLibrary: SharedLibrary<EhFrameHdr = Self>;

    /// The associated `.eh_frame` section type for this section.
    type EhFrame: EhFrame<EhFrameHdr = Self>;
}

/// An `.eh_frame` section within a mapped segment.
pub trait EhFrame: NamedMemoryRange<<Self as EhFrame>::SharedLibrary> {
    /// The associated segment type for this `.eh_frame_hdr` section.
    type Segment: Segment<EhFrame = Self>;

    /// The associated shared library type for this `.eh_frame_hdr` section.
    type SharedLibrary: SharedLibrary<EhFrame = Self>;

    /// The associated `.eh_frame_hdr` section type for this section.
    type EhFrameHdr: EhFrameHdr<EhFrame = Self>;
}

/// A mapped segment in a shared library.
pub trait Segment: NamedMemoryRange<<Self as Segment>::SharedLibrary> {
    /// The associated shared library type for this segment.
    type SharedLibrary: SharedLibrary<Segment = Self>;

    /// The associated `.eh_frame_hdr` section type for this segment.
    type EhFrameHdr: EhFrameHdr<Segment = Self>;

    /// The associated `.eh_frame` section type for this segment.
    type EhFrame: EhFrame<Segment = Self>;
}

/// A trait representing a shared library that is loaded in this process.
pub trait SharedLibrary: Sized + Debug {
    /// The associated segment type for this shared library.
    type Segment: Segment<SharedLibrary = Self>;

    /// An iterator over this shared library's segments.
    type SegmentIter: Iterator<Item = Self::Segment>;

    /// The associated `.eh_frame_hdr` section type for this shared library.
    type EhFrameHdr: EhFrameHdr<SharedLibrary = Self>;

    /// The associated `.eh_frame` section type for this shared library.
    type EhFrame: EhFrame<SharedLibrary = Self>;

    /// Get the name of this shared library.
    fn name(&self) -> &CStr;

    /// Get the bias of this shared library.
    ///
    /// See the module documentation for details.
    fn virtual_memory_bias(&self) -> Bias;

    /// Get the segments that fall within this shared library.
    fn segments(&self) -> Self::SegmentIter;

    /// Get the mapped `.eh_frame_hdr` section for this shared library, if any
    /// exists.
    fn eh_frame_hdr(&self) -> Option<Self::EhFrameHdr>;

    /// Get the mapped `.eh_frame` section for this shared library, if any
    /// exists.
    fn eh_frame(&self) -> Option<Self::EhFrame>;

    /// Iterate over the shared libraries mapped into this process and invoke
    /// `f` with each one.
    ///
    /// For example, to go through each shared library and do something with the
    /// shared library containing a given address, we might write a function
    /// like this:
    ///
    /// ```
    /// use findshlibs::{Avma, NamedMemoryRange, SharedLibrary, TargetSharedLibrary};
    /// use findshlibs::IterationControl::*;
    ///
    /// // Invoke `f` with the shared library containing `addr`, if any.
    /// fn with_shlib_containing_addr<F>(addr: *const u8, mut f: F)
    /// where
    ///     F: FnMut(&TargetSharedLibrary)
    /// {
    ///     let addr = Avma(addr);
    ///
    ///     TargetSharedLibrary::each(|shlib| {
    ///         for segment in shlib.segments() {
    ///             if segment.contains_avma(shlib, addr) {
    ///                 f(shlib);
    ///                 return Break;
    ///             }
    ///         }
    ///
    ///         Continue
    ///     });
    /// }
    /// ```
    ///
    /// If you don't need to early break out of a `SharedLibrary::each` loop,
    /// you don't need to worry about the `IterationControl` type, because of
    /// the `impl From<()> for IterationControl`. Just omit explicitly returning
    /// a value from the function, and iteration will always continue.
    fn each<F, C>(f: F)
    where
        F: FnMut(&Self) -> C,
        C: Into<IterationControl>;
}

/// Control whether iteration over shared libraries should continue or stop.
///
/// If you don't need to early break out of a `SharedLibrary::each` loop, you
/// don't need to worry about this type, because of the `impl From<()> for
/// IterationControl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IterationControl {
    /// Stop iteration.
    Break,
    /// Continue iteration.
    Continue,
}

impl From<()> for IterationControl {
    #[inline]
    fn from(_: ()) -> Self {
        IterationControl::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_in_each() {
        use std::panic;

        match panic::catch_unwind(|| {
            TargetSharedLibrary::each(|_| panic!("uh oh"));
        }) {
            Ok(()) => panic!("Expected a panic, but didn't get one"),
            Err(any) => {
                assert!(
                    any.is::<&'static str>(),
                    "panic value should be a &'static str"
                );
                assert_eq!(*any.downcast_ref::<&'static str>().unwrap(), "uh oh");
            }
        }
    }
}
