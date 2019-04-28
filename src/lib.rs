//! # `findshlibs`
//!
//! Find the set of shared libraries currently loaded in this process with a
//! cross platform API.
//!
//! The API entry point is the `TargetSharedLibrary` type and its
//! `SharedLibrary::each` trait method implementation.
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
//!             println!("    {}: segment {}",
//!                      seg.actual_virtual_memory_address(shlib),
//!                      seg.name().to_string_lossy());
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
//! If a platform is not supported then a fallback implementation is used that
//! does nothing.  To see if your platform does something at runtime the
//! `TARGET_SUPPORTED` constant can be used.
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

#[cfg(target_os = "linux")]
extern crate libc;

use std::ffi::CStr;
use std::fmt::{self, Debug};
use std::ptr;

pub mod unsupported;

cfg_if!(
    if #[cfg(target_os = "linux")] {

        pub mod linux;

        /// The [`SharedLibrary` trait](./trait.SharedLibrary.html)
        /// implementation for the target operating system.
        pub type TargetSharedLibrary<'a> = linux::SharedLibrary<'a>;

        /// An indicator if this platform is supported.
        pub const TARGET_SUPPORTED: bool = true;

    } else if #[cfg(target_os = "macos")] {

        pub mod macos;

        /// The [`SharedLibrary` trait](./trait.SharedLibrary.html)
        /// implementation for the target operating system.
        pub type TargetSharedLibrary<'a> = macos::SharedLibrary<'a>;

        /// An indicator if this platform is supported.
        pub const TARGET_SUPPORTED: bool = true;

    } else {

        /// The [`SharedLibrary` trait](./trait.SharedLibrary.html)
        /// implementation for the target operating system.
        pub type TargetSharedLibrary<'a> = unsupported::SharedLibrary<'a>;

        /// An indicator if this platform is supported.
        pub const TARGET_SUPPORTED: bool = false;

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
        display = "{:p}";

    /// Actual virtual memory address.
    ///
    /// See the module documentation for details.
    type Avma = *const u8
    where
        default = ptr::null(),
        display = "{:p}";

    /// Virtual memory bias.
    ///
    /// See the module documentation for details.
    type Bias = isize
    where
        default = 0,
        display = "{:#x}";
}

/// A mapped segment in a shared library.
#[allow(clippy::len_without_is_empty)]
pub trait Segment: Sized + Debug {
    /// The associated shared library type for this segment.
    type SharedLibrary: SharedLibrary<Segment = Self>;

    /// Get this segment's name.
    fn name(&self) -> &CStr;

    /// Returns `true` if this is a code segment.
    #[inline]
    fn is_code(&self) -> bool {
        false
    }

    /// Get this segment's stated virtual address of this segment.
    ///
    /// This is the virtual memory address without the bias applied. See the
    /// module documentation for details.
    fn stated_virtual_memory_address(&self) -> Svma;

    /// Get the length of this segment in memory (in bytes).
    fn len(&self) -> usize;

    // Provided methods.

    /// Get this segment's actual virtual memory address.
    ///
    /// This is the virtual memory address with the bias applied. See the module
    /// documentation for details.
    #[inline]
    fn actual_virtual_memory_address(&self, shlib: &Self::SharedLibrary) -> Avma {
        let svma = self.stated_virtual_memory_address();
        let bias = shlib.virtual_memory_bias();
        Avma(unsafe { svma.0.offset(bias.0) })
    }

    /// Does this segment contain the given address?
    #[inline]
    fn contains_svma(&self, address: Svma) -> bool {
        let start = self.stated_virtual_memory_address().0 as usize;
        let end = start + self.len();
        let address = address.0 as usize;
        start <= address && address < end
    }

    /// Does this segment contain the given address?
    #[inline]
    fn contains_avma(&self, shlib: &Self::SharedLibrary, address: Avma) -> bool {
        let start = self.actual_virtual_memory_address(shlib).0 as usize;
        let end = start + self.len();
        let address = address.0 as usize;
        start <= address && address < end
    }
}

/// Represents an ID for a shared library.
#[derive(PartialEq, Eq, Hash)]
pub enum SharedLibraryId {
    /// A UUID (used on mac)
    Uuid([u8; 16]),
    /// A GNU build ID
    GnuBuildId(Vec<u8>),
}

impl SharedLibraryId {
    /// Returns the raw bytes of the shared library ID.
    pub fn as_bytes(&self) -> &[u8] {
        match *self {
            SharedLibraryId::Uuid(ref bytes) => &*bytes,
            SharedLibraryId::GnuBuildId(ref bytes) => &bytes,
        }
    }
}

impl fmt::Display for SharedLibraryId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (bytes, is_uuid): (&[u8], _) = match *self {
            SharedLibraryId::Uuid(ref bytes) => (&*bytes, true),
            SharedLibraryId::GnuBuildId(ref bytes) => (&bytes, false),
        };
        for (idx, byte) in bytes.iter().enumerate() {
            if is_uuid && (idx == 4 || idx == 6 || idx == 8 || idx == 10) {
                try!(write!(f, "-"));
            }
            try!(write!(f, "{:02x}", byte));
        }
        Ok(())
    }
}

impl fmt::Debug for SharedLibraryId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SharedLibraryId::Uuid(..) => {
                write!(f, "Uuid(\"{}\")", self)?;
            }
            SharedLibraryId::GnuBuildId(..) => {
                write!(f, "GnuBuildId(\"{}\")", self)?;
            }
        }
        Ok(())
    }
}

/// A trait representing a shared library that is loaded in this process.
pub trait SharedLibrary: Sized + Debug {
    /// The associated segment type for this shared library.
    type Segment: Segment<SharedLibrary = Self>;

    /// An iterator over a shared library's segments.
    type SegmentIter: Debug + Iterator<Item = Self::Segment>;

    /// Get the name of this shared library.
    fn name(&self) -> &CStr;

    /// Get the debug-id of this shared library if available.
    fn id(&self) -> Option<SharedLibraryId>;

    /// Iterate over this shared library's segments.
    fn segments(&self) -> Self::SegmentIter;

    /// Get the bias of this shared library.
    ///
    /// See the module documentation for details.
    fn virtual_memory_bias(&self) -> Bias;

    /// Given an AVMA within this shared library, convert it back to an SVMA by
    /// removing this shared library's bias.
    #[inline]
    fn avma_to_svma(&self, address: Avma) -> Svma {
        let bias = self.virtual_memory_bias();
        let reverse_bias = -bias.0;
        Svma(unsafe { address.0.offset(reverse_bias) })
    }

    /// Find all shared libraries in this process and invoke `f` with each one.
    fn each<F, C>(f: F)
    where
        F: FnMut(&Self) -> C,
        C: Into<IterationControl>;
}

/// Control whether iteration over shared libraries should continue or stop.
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
