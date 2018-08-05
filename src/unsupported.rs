//! The fallback implementation of the [SharedLibrary
//! trait](../trait.SharedLibrary.html) that does nothing.

use super::Segment as SegmentTrait;
use super::SharedLibrary as SharedLibraryTrait;
use super::{Bias, IterationControl, SharedLibraryId, Svma, EhFrame, EhFrameHdr, NamedMemoryRange};

use std::ffi::CStr;
use std::marker::PhantomData;
use std::usize;

/// An unsupported segment
#[derive(Debug)]
pub struct Segment<'a> {
    phantom: PhantomData<&'a SharedLibrary<'a>>,
}

impl <'a> SegmentTrait for Segment<'a> {
    type SharedLibrary = SharedLibrary<'a>;
    type EhFrameHdr = NoOpEhFrameHdr<'a>;
    type EhFrame = NoOpEhFrame<'a>;
}

impl <'a> NamedMemoryRange<SharedLibrary<'a>> for Segment<'a> {
    fn name(&self) -> &'_ CStr {
        unreachable!()
    }

    fn stated_virtual_memory_address(&self) -> Svma {
        unreachable!()
    }

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

/// Unsuppoered EhFrame
#[derive(Debug)]
pub struct NoOpEhFrame<'a> {
    phantom: PhantomData<&'a SharedLibrary<'a>>,
}

impl<'a> EhFrame for NoOpEhFrame<'a> {
    type Segment = Segment<'a>;
    type SharedLibrary = SharedLibrary<'a>;
    type EhFrameHdr = NoOpEhFrameHdr<'a>;
}

impl <'a> NamedMemoryRange<SharedLibrary<'a>> for NoOpEhFrame<'a> {
    fn name(&self) -> &'_ CStr {
        unreachable!()
    }

    fn stated_virtual_memory_address(&self) -> Svma {
        unreachable!()
    }

    fn len(&self) -> usize {
        unreachable!()
    }
}

/// Unsupported EhFrameHdr
#[derive(Debug)]
pub struct NoOpEhFrameHdr<'a> {
    phantom: PhantomData<&'a SharedLibrary<'a>>,
}

impl <'a> EhFrameHdr for NoOpEhFrameHdr<'a> {
    type Segment = Segment<'a>;
    type SharedLibrary = SharedLibrary<'a>;
    type EhFrame = NoOpEhFrame<'a>;
}

impl <'a> NamedMemoryRange<SharedLibrary<'a>> for NoOpEhFrameHdr<'a> {
    fn name(&self) -> &'_ CStr {
        unreachable!()
    }

    fn stated_virtual_memory_address(&self) -> Svma {
        unreachable!()
    }

    fn len(&self) -> usize {
        unreachable!()
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
    type EhFrameHdr = NoOpEhFrameHdr<'a>;
    type EhFrame = NoOpEhFrame<'a>;

    #[inline]
    fn name(&self) -> &CStr {
        unreachable!()
    }

    fn id(&self) -> Option<SharedLibraryId> {
        unreachable!()
    }

    fn segments(&self) -> Self::SegmentIter {
        SegmentIter {
            phantom: PhantomData,
        }
    }

    #[inline]
    fn virtual_memory_bias(&self) -> Bias {
        unreachable!()
    }

    fn each<F, C>(_f: F)
    where
        F: FnMut(&Self) -> C,
        C: Into<IterationControl>,
    {
    }

    fn eh_frame_hdr(&self) -> Option<Self::EhFrameHdr> {
        unreachable!()
    }

    fn eh_frame(&self) -> Option<Self::EhFrame> {
        unreachable!()
    }
}
