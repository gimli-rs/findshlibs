//! Linux-specific implementation of the `SharedLibrary` trait.

use shared_lib;

use std::ffi::CStr;
use std::slice;

mod bindings;

/// A shared library on Linux.
#[derive(Debug, Clone, Copy)]
pub struct SharedLibrary<'a> {
    size: usize,
    addr: *const u8,
    name: &'a CStr,
    // TODO FITZGEN: 32 bit too?
    headers: &'a [bindings::Elf64_Phdr],
    // TODO FITZGEN: other fields?
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
              C: Into<shared_lib::IterationControl>
    {
        let f = f as *mut F;
        let f = f.as_mut().unwrap();

        let info = info.as_ref().unwrap();
        let shlib = SharedLibrary::new(info, size);

        match f(&shlib).into() {
            shared_lib::IterationControl::Break => 1,
            shared_lib::IterationControl::Continue => 0,
        }
    }
}

impl<'a> shared_lib::SharedLibrary for SharedLibrary<'a> {
    fn name(&self) -> &CStr {
        self.name
    }

    fn each<F, C>(mut f: F)
        where F: FnMut(&Self) -> C,
              C: Into<shared_lib::IterationControl>
    {
        unsafe {
            bindings::dl_iterate_phdr(Some(Self::callback::<F, C>), &mut f as *mut _ as *mut _);
        }
    }
}

#[cfg(test)]
mod tests {
    use linux;
    use shared_lib::{IterationControl, SharedLibrary};

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
}
