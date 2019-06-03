//! Linux-specific implementation of the `SharedLibrary` trait.

use super::Segment as SegmentTrait;
use super::SharedLibrary as SharedLibraryTrait;
use super::{Bias, IterationControl, SharedLibraryId, Svma};

use std::any::Any;
use std::borrow::Cow;
use std::env::current_exe;
use std::ffi::{CStr, CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::fmt;
use std::isize;
use std::marker::PhantomData;
use std::mem;
use std::os::unix::ffi::OsStringExt;
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

const NT_GNU_BUILD_ID: u32 = 3;

struct Nhdr32 {
    pub n_namesz: libc::Elf32_Word,
    pub n_descsz: libc::Elf32_Word,
    pub n_type: libc::Elf32_Word,
}

/// A mapped segment in an ELF file.
#[derive(Debug)]
pub struct Segment<'a> {
    phdr: *const Phdr,
    shlib: PhantomData<&'a ::linux::SharedLibrary<'a>>,
}

impl<'a> SegmentTrait for Segment<'a> {
    type SharedLibrary = ::linux::SharedLibrary<'a>;

    fn name(&self) -> &str {
        unsafe {
            match self.phdr.as_ref().unwrap().p_type {
                libc::PT_NULL => "NULL",
                libc::PT_LOAD => "LOAD",
                libc::PT_DYNAMIC => "DYNAMIC",
                libc::PT_INTERP => "INTERP",
                libc::PT_NOTE => "NOTE",
                libc::PT_SHLIB => "SHLI",
                libc::PT_PHDR => "PHDR",
                libc::PT_TLS => "TLS",
                libc::PT_NUM => "NUM",
                libc::PT_LOOS => "LOOS",
                libc::PT_GNU_EH_FRAME => "GNU_EH_FRAME",
                libc::PT_GNU_STACK => "GNU_STACK",
                libc::PT_GNU_RELRO => "GNU_RELRO",
                _ => "(unknown segment type)",
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
        Svma(unsafe { (*self.phdr).p_vaddr as _ })
    }

    #[inline]
    fn len(&self) -> usize {
        unsafe { (*self.phdr).p_memsz as _ }
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
            shlib: PhantomData,
        })
    }
}

impl<'a> fmt::Debug for SegmentIter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ref phdr = self.inner.as_slice()[0];

        f.debug_struct("SegmentIter")
            .field("phdr", &DebugPhdr(phdr))
            .finish()
    }
}

/// A shared library on Linux.
pub struct SharedLibrary<'a> {
    size: usize,
    addr: *const u8,
    name: Cow<'a, CStr>,
    headers: &'a [Phdr],
}

struct IterState<F> {
    f: F,
    panic: Option<Box<Any + Send>>,
    idx: usize,
}

const CONTINUE: libc::c_int = 0;
const BREAK: libc::c_int = 1;

impl<'a> SharedLibrary<'a> {
    unsafe fn new(info: &'a libc::dl_phdr_info, size: usize, is_first_lib: bool) -> Self {
        // try to get the name from the dl_phdr_info.  If that fails there are two
        // cases we can and need to deal with.  The first one is if we are the first
        // loaded library in which case the name is the executable which we can
        // discover via env::current_exe (reads the proc/self symlink).
        //
        // Otherwise if we have a no name we might be a dylib that was loaded with
        // dlopen in which case we can use dladdr to recover the name.
        let mut name = Cow::Borrowed(if info.dlpi_name.is_null() {
            CStr::from_bytes_with_nul_unchecked(b"\0")
        } else {
            CStr::from_ptr(info.dlpi_name)
        });
        if name.to_bytes().is_empty() {
            if is_first_lib {
                if let Ok(exe) = current_exe() {
                    name = Cow::Owned(CString::from_vec_unchecked(exe.into_os_string().into_vec()));
                }
            } else {
                let mut dlinfo: libc::Dl_info = mem::zeroed();
                if libc::dladdr(info.dlpi_addr as *const libc::c_void, &mut dlinfo) != 0 {
                    name = Cow::Owned(CString::from(CStr::from_ptr(dlinfo.dli_fname)));
                }
            }
        }

        SharedLibrary {
            size: size,
            addr: info.dlpi_addr as usize as *const _,
            name,
            headers: slice::from_raw_parts(info.dlpi_phdr, info.dlpi_phnum as usize),
        }
    }

    unsafe extern "C" fn callback<F, C>(
        info: *mut libc::dl_phdr_info,
        size: usize,
        state: *mut libc::c_void,
    ) -> libc::c_int
    where
        F: FnMut(&Self) -> C,
        C: Into<IterationControl>,
    {
        let state = &mut *(state as *mut IterState<F>);
        state.idx += 1;

        match panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let info = info.as_ref().unwrap();
            let shlib = SharedLibrary::new(info, size, state.idx == 1);

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
    fn name(&self) -> &OsStr {
        OsStr::from_bytes(self.name.to_bytes())
    }

    fn id(&self) -> Option<SharedLibraryId> {
        fn align(alignment: usize, offset: &mut usize) {
            let diff = *offset % alignment;
            if diff != 0 {
                *offset += alignment - diff;
            }
        }

        unsafe {
            for segment in self.segments() {
                let phdr = segment.phdr.as_ref().unwrap();
                if phdr.p_type != libc::PT_NOTE {
                    continue;
                }

                let mut alignment = phdr.p_align as usize;
                // same logic as in gimli which took it from readelf
                if alignment < 4 {
                    alignment = 4;
                } else if alignment != 4 && alignment != 8 {
                    continue;
                }

                let mut offset = phdr.p_offset as usize;
                let end = offset + phdr.p_filesz as usize;

                while offset < end {
                    // we always use an nhdr32 here as 64bit notes have not
                    // been observed in practice.
                    let nhdr = &*((self.addr as usize + offset) as *const Nhdr32);
                    offset += mem::size_of_val(nhdr);
                    offset += nhdr.n_namesz as usize;
                    align(alignment, &mut offset);
                    let value =
                        slice::from_raw_parts(self.addr.add(offset), nhdr.n_descsz as usize);
                    offset += nhdr.n_descsz as usize;
                    align(alignment, &mut offset);

                    if nhdr.n_type as u32 == NT_GNU_BUILD_ID {
                        return Some(SharedLibraryId::GnuBuildId(value.to_vec()));
                    }
                }
            }
        }

        None
    }

    #[inline]
    fn segments(&self) -> Self::SegmentIter {
        SegmentIter {
            inner: self.headers.iter(),
        }
    }

    #[inline]
    fn virtual_memory_bias(&self) -> Bias {
        assert!((self.addr as usize) < (isize::MAX as usize));
        Bias(self.addr as usize as isize)
    }

    #[inline]
    fn each<F, C>(f: F)
    where
        F: FnMut(&Self) -> C,
        C: Into<IterationControl>,
    {
        let mut state = IterState {
            f: f,
            panic: None,
            idx: 0,
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
        write!(
            f,
            "SharedLibrary {{ size: {:?}, addr: {:?}, ",
            self.size, self.addr
        )?;
        write!(f, "name: {:?}, headers: [", self.name)?;

        // Debug does not usually have a trailing comma in the list,
        // last element must be formatted separately.
        let l = self.headers.len();
        self.headers[..(l - 1)]
            .into_iter()
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
    use super::super::{IterationControl, Segment, SharedLibrary};
    use linux;

    #[test]
    fn have_libc() {
        let mut found_libc = false;
        linux::SharedLibrary::each(|info| {
            found_libc |= info
                .name
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
        use std::ffi::OsStr;
        let mut names = vec![];
        linux::SharedLibrary::each(|shlib| {
            println!("{:?}", shlib);
            let name = shlib.name();
            if name != OsStr::new("") {
                names.push(name.to_str().unwrap().to_string());
            }
        });

        assert!(names[0].contains("/findshlibs"));
        assert!(names.iter().any(|x| x.contains("libc.so")));
    }

    #[test]
    fn get_id() {
        use std::path::Path;
        use std::process::Command;

        linux::SharedLibrary::each(|shlib| {
            let name = shlib.name();
            let id = shlib.id();
            if id.is_none() {
                println!("no id found for {:?}", name);
                return;
            }
            let path: &Path = name.as_ref();
            if !path.is_absolute() {
                return;
            }
            let gnu_build_id = id.unwrap().to_string();
            let readelf = Command::new("readelf")
                .arg("-n")
                .arg(path)
                .output()
                .unwrap();
            for line in String::from_utf8(readelf.stdout).unwrap().lines() {
                if let Some(index) = line.find("Build ID: ") {
                    let readelf_build_id = line[index + 9..].trim();
                    assert_eq!(readelf_build_id, gnu_build_id);
                }
            }
            println!("{}: {}", path.display(), gnu_build_id);
        });
    }

    #[test]
    fn have_load_segment() {
        linux::SharedLibrary::each(|shlib| {
            println!("shlib = {:?}", shlib.name());

            let mut found_load = false;
            for seg in shlib.segments() {
                println!("    segment = {:?}", seg.name());

                found_load |= seg.name() == "LOAD";
            }
            assert!(found_load);
        });
    }
}
