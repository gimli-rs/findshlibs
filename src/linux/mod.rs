//! Linux-specific implementation of the `SharedLibrary` trait.

use super::{Bias, IterationControl, NamedMemoryRange, Svma, SharedLibraryId};
use super::EhFrameHdr as EhFrameHdrTrait;
use super::Segment as SegmentTrait;
use super::SharedLibrary as SharedLibraryTrait;

use std::any::Any;
use std::ffi::CStr;
use std::isize;
use std::marker::PhantomData;
use std::os::raw;
use std::panic;
use std::slice;

mod bindings;

cfg_if! {
    if #[cfg(target_pointer_width = "32")] {
        type Phdr = bindings::Elf32_Phdr;
    } else if #[cfg(target_pointer_width = "64")] {
        type Phdr = bindings::Elf64_Phdr;
    } else {
        compile_error!("Unsupported architecture; only 32 and 64 bit pointer \
                        widths are supported");
    }
}

/// A mapped `.eh_frame_hdr` section.
#[derive(Debug)]
pub struct EhFrameHdr<'a> {
    svma: Svma,
    len: usize,
    shlib: PhantomData<&'a SharedLibrary<'a>>,
}

impl<'a> EhFrameHdrTrait for EhFrameHdr<'a> {
    type Segment = Segment<'a>;
    type SharedLibrary = SharedLibrary<'a>;
}

impl<'a> NamedMemoryRange<SharedLibrary<'a>> for EhFrameHdr<'a> {
    #[inline]
    fn name(&self) -> &CStr {
        unsafe {
            CStr::from_ptr(".eh_frame_hdr\0".as_ptr() as _)
        }
    }

    #[inline]
    fn stated_virtual_memory_address(&self) -> Svma {
        self.svma
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }
}

/// A mapped segment in an ELF file.
#[derive(Debug)]
pub struct Segment<'a> {
    phdr: *const Phdr,
    shlib: PhantomData<&'a SharedLibrary<'a>>,
}

impl<'a> NamedMemoryRange<SharedLibrary<'a>> for Segment<'a> {
    fn name(&self) -> &CStr {
        unsafe {
            match self.phdr.as_ref().unwrap().p_type {
                bindings::PT_NULL => CStr::from_ptr("NULL\0".as_ptr() as _),
                bindings::PT_LOAD => CStr::from_ptr("LOAD\0".as_ptr() as _),
                bindings::PT_DYNAMIC => CStr::from_ptr("DYNAMIC\0".as_ptr() as _),
                bindings::PT_INTERP => CStr::from_ptr("INTERP\0".as_ptr() as _),
                bindings::PT_NOTE => CStr::from_ptr("NOTE\0".as_ptr() as _),
                bindings::PT_SHLIB => CStr::from_ptr("SHLI\0".as_ptr() as _),
                bindings::PT_PHDR => CStr::from_ptr("PHDR\0".as_ptr() as _),
                bindings::PT_TLS => CStr::from_ptr("TLS\0".as_ptr() as _),
                bindings::PT_NUM => CStr::from_ptr("NUM\0".as_ptr() as _),
                bindings::PT_LOOS => CStr::from_ptr("LOOS\0".as_ptr() as _),
                bindings::PT_GNU_EH_FRAME => CStr::from_ptr("GNU_EH_FRAME\0".as_ptr() as _),
                bindings::PT_GNU_STACK => CStr::from_ptr("GNU_STACK\0".as_ptr() as _),
                bindings::PT_GNU_RELRO => CStr::from_ptr("GNU_RELRO\0".as_ptr() as _),
                _ => CStr::from_ptr("(unknown segment type)\0".as_ptr() as _),
            }
        }
    }

    #[inline]
    fn stated_virtual_memory_address(&self) -> Svma {
        Svma(unsafe {
            (*self.phdr).p_vaddr as _
        })
    }

    #[inline]
    fn len(&self) -> usize {
        unsafe {
            (*self.phdr).p_memsz as _
        }
    }
}

impl<'a> SegmentTrait for Segment<'a> {
    type SharedLibrary = SharedLibrary<'a>;
    type EhFrameHdr = EhFrameHdr<'a>;
}

/// An iterator of mapped segments in a shared library.
#[derive(Debug)]
pub struct SegmentIter<'a> {
    inner: ::std::slice::Iter<'a, Phdr>,
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = Segment<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|phdr| Segment {
            phdr: phdr,
            shlib: PhantomData
        })
    }
}

/// A shared library on Linux.
#[derive(Debug, Clone, Copy)]
pub struct SharedLibrary<'a> {
    size: usize,
    addr: *const u8,
    name: &'a CStr,
    headers: &'a [Phdr],
}

struct IterState<F> {
    f: F,
    panic: Option<Box<Any + Send>>,
}

const CONTINUE: raw::c_int = 0;
const BREAK: raw::c_int = 1;

impl<'a> SharedLibrary<'a> {
    unsafe fn new(info: &'a bindings::dl_phdr_info, size: usize) -> Self {
        SharedLibrary {
            size: size,
            addr: info.dlpi_addr as usize as *const _,
            name: CStr::from_ptr(info.dlpi_name),
            headers: slice::from_raw_parts(info.dlpi_phdr, info.dlpi_phnum as usize),
        }
    }

    unsafe extern "C" fn callback<F, C>(info: *mut bindings::dl_phdr_info,
                                        size: usize,
                                        state: *mut raw::c_void)
                                        -> raw::c_int
        where F: FnMut(&Self) -> C,
              C: Into<IterationControl>
    {
        let state = &mut *(state as *mut IterState<F>);

        match panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let info = info.as_ref().unwrap();
            let shlib = SharedLibrary::new(info, size);

            (state.f)(&shlib).into()
        })) {
            Ok(IterationControl::Continue) => CONTINUE,
            Ok(IterationControl::Break) => BREAK,
            Err(panicked) => {
                state.panic = Some(panicked);
                BREAK
            }
        }
    }
}

impl<'a> SharedLibraryTrait for SharedLibrary<'a> {
    type Segment = Segment<'a>;
    type SegmentIter = SegmentIter<'a>;
    type EhFrameHdr = EhFrameHdr<'a>;

    #[inline]
    fn name(&self) -> &CStr {
        self.name
    }

    #[inline]
    fn id(&self) -> Option<SharedLibraryId> {
        None
    }

    #[inline]
    fn segments(&self) -> Self::SegmentIter {
        SegmentIter { inner: self.headers.iter() }
    }

    #[inline]
    fn eh_frame_hdr(&self) -> Option<Self::EhFrameHdr> {
        for seg in self.segments() {
            let phdr = unsafe {
                seg.phdr.as_ref().unwrap()
            };

            if phdr.p_type == bindings::PT_GNU_EH_FRAME {
                return Some(EhFrameHdr {
                    svma: Svma(phdr.p_vaddr as _),
                    len: phdr.p_memsz as _,
                    shlib: PhantomData,
                });
            }
        }

        None
    }

    #[inline]
    fn virtual_memory_bias(&self) -> Bias {
        assert!((self.addr as usize) < (isize::MAX as usize));
        Bias(self.addr as usize as isize)
    }

    #[inline]
    fn each<F, C>(f: F)
        where F: FnMut(&Self) -> C,
              C: Into<IterationControl>
    {
        let mut state = IterState {
            f: f,
            panic: None,
        };

        unsafe {
            bindings::dl_iterate_phdr(Some(Self::callback::<F, C>), &mut state as *mut _ as *mut _);
        }

        if let Some(panic) = state.panic {
            panic::resume_unwind(panic);
        }
    }
}

#[cfg(test)]
mod tests {
    use linux;
    use super::super::{IterationControl, NamedMemoryRange, SharedLibrary};

    #[test]
    fn have_libc() {
        let mut found_libc = false;
        linux::SharedLibrary::each(|info| {
            found_libc |= info.name
                .to_bytes()
                .split(|c| *c == b'.' || *c == b'/')
                .find(|s| s == b"libc")
                .is_some();
        });
        assert!(found_libc);
    }

    #[test]
    fn can_break() {
        let mut first_count = 0;
        linux::SharedLibrary::each(|_| {
            first_count += 1;
        });
        assert!(first_count > 2);

        let mut second_count = 0;
        linux::SharedLibrary::each(|_| {
            second_count += 1;

            if second_count == first_count - 1 {
                IterationControl::Break
            } else {
                IterationControl::Continue
            }
        });
        assert_eq!(second_count, first_count - 1);
    }

    #[test]
    fn get_name() {
        linux::SharedLibrary::each(|shlib| {
            let _ = shlib.name();
        });
    }

    #[test]
    fn have_load_segment() {
        linux::SharedLibrary::each(|shlib| {
            println!("shlib = {:?}", shlib.name());

            let mut found_load = false;
            for seg in shlib.segments() {
                println!("    segment = {:?}", seg.name());

                found_load |= seg.name().to_bytes() == b"LOAD";
            }
            assert!(found_load);
        });
    }
}
