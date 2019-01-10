//! Linux-specific implementation of the `SharedLibrary` trait.

use super::{Bias, IterationControl, Svma, SharedLibraryId};
use super::Segment as SegmentTrait;
use super::SharedLibrary as SharedLibraryTrait;

use std::any::Any;
use std::ffi::CStr;
use std::fmt;
use std::isize;
use std::marker::PhantomData;
use std::panic;
use std::slice;

use libc;

cfg_if! {
    if #[cfg(target_pointer_width = "32")] {
        type Phdr = libc::Elf32_Phdr;
    } else if #[cfg(target_pointer_width = "64")] {
        type Phdr = libc::Elf64_Phdr;
    } else {
        // Unsupported.
    }
}

/// A mapped segment in an ELF file.
#[derive(Debug)]
pub struct Segment<'a> {
    phdr: *const Phdr,
    shlib: PhantomData<&'a ::linux::SharedLibrary<'a>>,
}

impl<'a> SegmentTrait for Segment<'a> {
    type SharedLibrary = ::linux::SharedLibrary<'a>;

    fn name(&self) -> &CStr {
        unsafe {
            match self.phdr.as_ref().unwrap().p_type {
                libc::PT_NULL => CStr::from_ptr("NULL\0".as_ptr() as _),
                libc::PT_LOAD => CStr::from_ptr("LOAD\0".as_ptr() as _),
                libc::PT_DYNAMIC => CStr::from_ptr("DYNAMIC\0".as_ptr() as _),
                libc::PT_INTERP => CStr::from_ptr("INTERP\0".as_ptr() as _),
                libc::PT_NOTE => CStr::from_ptr("NOTE\0".as_ptr() as _),
                libc::PT_SHLIB => CStr::from_ptr("SHLI\0".as_ptr() as _),
                libc::PT_PHDR => CStr::from_ptr("PHDR\0".as_ptr() as _),
                libc::PT_TLS => CStr::from_ptr("TLS\0".as_ptr() as _),
                libc::PT_NUM => CStr::from_ptr("NUM\0".as_ptr() as _),
                libc::PT_LOOS => CStr::from_ptr("LOOS\0".as_ptr() as _),
                libc::PT_GNU_EH_FRAME => CStr::from_ptr("GNU_EH_FRAME\0".as_ptr() as _),
                libc::PT_GNU_STACK => CStr::from_ptr("GNU_STACK\0".as_ptr() as _),
                libc::PT_GNU_RELRO => CStr::from_ptr("GNU_RELRO\0".as_ptr() as _),
                _ => CStr::from_ptr("(unknown segment type)\0".as_ptr() as _),
            }
        }
    }

    #[inline]
    fn is_code(&self) -> bool {
        unsafe {
            let hdr = self.phdr.as_ref().unwrap();
            match hdr.p_type {
                // 0x1 is PT_X for executable
                libc::PT_LOAD => (hdr.p_flags & 0x1) != 0,
                _ => false,
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

/// An iterator of mapped segments in a shared library.
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

impl<'a> fmt::Debug for SegmentIter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ref phdr = self.inner.as_slice()[0];

        f.debug_struct("SegmentIter").field("phdr", &DebugPhdr(phdr)).finish()
    }
}

/// A shared library on Linux.
#[derive(Clone, Copy)]
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

const CONTINUE: libc::c_int = 0;
const BREAK: libc::c_int = 1;

impl<'a> SharedLibrary<'a> {
    unsafe fn new(info: &'a libc::dl_phdr_info, size: usize) -> Self {
        SharedLibrary {
            size: size,
            addr: info.dlpi_addr as usize as *const _,
            name: CStr::from_ptr(info.dlpi_name),
            headers: slice::from_raw_parts(info.dlpi_phdr, info.dlpi_phnum as usize),
        }
    }

    unsafe extern "C" fn callback<F, C>(info: *mut libc::dl_phdr_info,
                                        size: usize,
                                        state: *mut libc::c_void)
                                        -> libc::c_int
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
            libc::dl_iterate_phdr(Some(Self::callback::<F, C>), &mut state as *mut _ as *mut _);
        }

        if let Some(panic) = state.panic {
            panic::resume_unwind(panic);
        }
    }
}

impl<'a> fmt::Debug for SharedLibrary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SharedLibrary {{ size: {:?}, addr: {:?}, ", self.size, self.addr)?;
        write!(f, "name: {:?}, headers: [",  self.name)?;

        // Debug does not usually have a trailing comma in the list,
        // last element must be formatted separately.
        let l = self.headers.len();
        self.headers[..(l - 1)].into_iter()
            .map(|phdr| write!(f, "{:?}, ", &DebugPhdr(phdr)))
            .collect::<fmt::Result>()?;

        write!(f, "{:?}", &DebugPhdr(&self.headers[l - 1]))?;

        write!(f, "] }}")
    }
}

struct DebugPhdr<'a>(&'a Phdr);

impl<'a> fmt::Debug for DebugPhdr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let phdr = self.0;

        // The layout is different for 32-bit vs 64-bit,
        // but since the fields are the same, it shouldn't matter much.
        f.debug_struct("Phdr")
            .field("p_type", &phdr.p_type)
            .field("p_flags", &phdr.p_flags)
            .field("p_offset", &phdr.p_offset)
            .field("p_vaddr", &phdr.p_vaddr)
            .field("p_paddr", &phdr.p_paddr)
            .field("p_filesz", &phdr.p_filesz)
            .field("p_memsz", &phdr.p_memsz)
            .field("p_align", &phdr.p_align)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use linux;
    use super::super::{IterationControl, SharedLibrary, Segment};

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
            println!("{:?}", shlib);
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
