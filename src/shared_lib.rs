use std::ffi::CStr;

/// A trait representing a shared library that is loaded in this process.
pub trait SharedLibrary {
    /// Get this shared library's path name.
    fn name(&self) -> &CStr;

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
