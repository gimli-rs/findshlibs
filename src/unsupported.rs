//! The fallback implementation of the [SharedLibrary
//! trait](../trait.SharedLibrary.html) that does nothing.

use super::{Bias, IterationControl, Svma, SharedLibraryId};
use super::Segment as SegmentTrait;
use super::SharedLibrary as SharedLibraryTrait;

use std::ffi::CStr;
use std::marker::PhantomData;
use std::usize;

/// An unsupported segment
#[derive(Debug)]
pub struct Segment<'a> {
    phantom: PhantomData<&'a SharedLibrary<'a>>,
}

impl<'a> SegmentTrait for Segment<'a> {
    type SharedLibrary = ::unsupported::SharedLibrary<'a>;

    #[inline]
    fn name(&self) -> &CStr {
        unreachable!()
    }

    #[inline]
    fn stated_virtual_memory_address(&self) -> Svma {
        unreachable!()
    }

    #[inline]
    fn len(&self) -> usize {
        unreachable!()
    }
}

/// An iterator over Mach-O segments.
#[derive(Debug)]
pub struct SegmentIter<'a> {
    phantom: PhantomData<&'a SharedLibrary<'a>>,
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = Segment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}


/// The fallback implementation of the [SharedLibrary
/// trait](../trait.SharedLibrary.html).
#[derive(Debug)]
pub struct SharedLibrary<'a> {
    phantom: PhantomData<&'a SharedLibrary<'a>>,
}

impl<'a> SharedLibraryTrait for SharedLibrary<'a> {
    type Segment = Segment<'a>;
    type SegmentIter = SegmentIter<'a>;

    #[inline]
    fn name(&self) -> &CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(b"\x00") }
    }

    fn id(&self) -> Option<SharedLibraryId> {
        None
    }

    fn segments(&self) -> Self::SegmentIter {
        SegmentIter {
            phantom: PhantomData,
        }
    }

    #[inline]
    fn virtual_memory_bias(&self) -> Bias {
        Bias(0)
    }

    fn each<F, C>(_f: F)
        where F: FnMut(&Self) -> C,
              C: Into<IterationControl>
    {
    }
}
