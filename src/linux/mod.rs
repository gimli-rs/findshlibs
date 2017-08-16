//! Linux-specific implementation of the `SharedLibrary` trait.

use super::{Bias, IterationControl, Svma};
use super::Segment as SegmentTrait;
use super::SharedLibrary as SharedLibraryTrait;

use std::ffi::CStr;
use std::isize;
use std::marker::PhantomData;
use std::slice;

mod bindings;

cfg_if! {
    if #[cfg(target_pointer_width = "32")] {
        type Phdr = bindings::Elf32_Phdr;
    } else if #[cfg(target_pointer_width = "64")] {
        type Phdr = bindings::Elf64_Phdr;
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

    fn stated_virtual_memory_address(&self) -> Svma {
        Svma(unsafe {
            (*self.phdr).p_vaddr as _
        })
    }

    fn len(&self) -> usize {
        unsafe {
            (*self.phdr).p_memsz as _
        }
    }
}

/// An iterator of mapped segments in a shared library.
#[derive(Debug)]
pub struct SegmentIter<'a> {
    inner: ::std::slice::Iter<'a, Phdr>,
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = Segment<'a>;

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
                                        f: *mut ::std::os::raw::c_void)
                                        -> ::std::os::raw::c_int
        where F: FnMut(&Self) -> C,
              C: Into<IterationControl>
    {
        let f = f as *mut F;
        let f = f.as_mut().unwrap();

        let info = info.as_ref().unwrap();
        let shlib = SharedLibrary::new(info, size);

        match f(&shlib).into() {
            IterationControl::Break => 1,
            IterationControl::Continue => 0,
        }
    }
}

impl<'a> SharedLibraryTrait for SharedLibrary<'a> {
    type Segment = Segment<'a>;
    type SegmentIter = SegmentIter<'a>;

    fn name(&self) -> &CStr {
        self.name
    }

    fn segments(&self) -> Self::SegmentIter {
        SegmentIter { inner: self.headers.iter() }
    }

    fn virtual_memory_bias(&self) -> Bias {
        assert!((self.addr as usize) < (isize::MAX as usize));
        Bias(self.addr as usize as isize)
    }

    fn each<F, C>(mut f: F)
        where F: FnMut(&Self) -> C,
              C: Into<IterationControl>
    {
        unsafe {
            bindings::dl_iterate_phdr(Some(Self::callback::<F, C>), &mut f as *mut _ as *mut _);
        }
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
