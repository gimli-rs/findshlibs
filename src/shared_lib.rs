use std::ffi::CStr;
use std::fmt::Debug;

/// A mapped segment in a shared library.
pub trait Segment: Debug {
    /// Get this segment's name.
    fn name(&self) -> &CStr;
}

/// A trait representing a shared library that is loaded in this process.
pub trait SharedLibrary {
    /// The associated segment type for this shared library.
    type Segment: ::shared_lib::Segment;

    /// An iterator over a shared library's segments.
    type SegmentIter: Debug + Iterator<Item = Self::Segment>;

    /// Get the name of this shared library.
    fn name(&self) -> &CStr;

    /// Iterate over this shared library's segments.
    fn segments(&self) -> Self::SegmentIter;

    /// Find all shared libraries in this process and invoke `f` with each one.
    fn each<F, C>(f: F)
        where F: FnMut(&Self) -> C,
              C: Into<IterationControl>;
}

/// Control whether iteration over shared libraries should continue or stop.
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
