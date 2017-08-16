use std::ffi::CStr;
use std::fmt::Debug;

/// A mapped segment in a shared library.
pub trait Segment: Sized + Debug {
    /// The associated shared library type for this segment.
    type SharedLibrary: ::shared_lib::SharedLibrary<Segment = Self>;

    /// Get this segment's name.
    fn name(&self) -> &CStr;

    /// Get this segment's stated virtual address of this segment.
    ///
    /// This is the virtual memory address without the bias applied. See the
    /// module documentation for details.
    fn stated_virtual_memory_address(&self) -> usize;

    /// Get this segment's actual virtual memory address.
    ///
    /// This is the virtual memory address with the bias applied. See the module
    /// documentation for details.
    fn actual_virtual_memory_address(&self, shlib: &Self::SharedLibrary) -> usize;

    /// Get the length of this segment in memory (in bytes).
    fn len(&self) -> usize;
}

/// A trait representing a shared library that is loaded in this process.
pub trait SharedLibrary: Sized + Debug {
    /// The associated segment type for this shared library.
    type Segment: ::shared_lib::Segment<SharedLibrary = Self>;

    /// An iterator over a shared library's segments.
    type SegmentIter: Debug + Iterator<Item = Self::Segment>;

    /// Get the name of this shared library.
    fn name(&self) -> &CStr;

    /// Iterate over this shared library's segments.
    fn segments(&self) -> Self::SegmentIter;

    /// Get the bias of this shared library.
    ///
    /// See the module documentation for details.
    fn virtual_memory_bias(&self) -> usize;

    /// Find all shared libraries in this process and invoke `f` with each one.
    fn each<F, C>(f: F)
        where F: FnMut(&Self) -> C,
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
    fn from(_: ()) -> Self {
        IterationControl::Continue
    }
}
