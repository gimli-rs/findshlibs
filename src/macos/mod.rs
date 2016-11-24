//! The MacOS implementation of the [SharedLibrary
//! trait](../trait.SharedLibrary.html).

use shared_lib;
use std::ffi::CStr;
use std::ptr;
use std::sync::Mutex;

mod bindings;

lazy_static! {
    /// A lock protecting dyld FFI calls.
    ///
    /// MacOS does not provide an atomic way to iterate shared libraries, so
    /// *you* must take this lock whenever dynamically adding or removing shared
    /// libraries to ensure that there are no races with iterating shared
    /// libraries.
    pub static ref DYLD_LOCK: Mutex<()> = Mutex::new(());
}

/// The MacOS implementation of the [SharedLibrary
/// trait](../trait.SharedLibrary.html).
///
/// This wraps the `_dyld_image_count` and
/// `_dyld_get_image_{header,vmaddr_slide,name}` system APIs from the
/// `<mach-o/dyld.h>` header.
#[derive(Debug)]
pub struct SharedLibrary<'a> {
    header: &'a bindings::mach_header,
    slide: isize,
    name: &'a CStr,
}

impl<'a> SharedLibrary<'a> {
    fn new(header: &'a bindings::mach_header, slide: isize, name: &'a CStr) -> Self {
        SharedLibrary {
            header: header,
            slide: slide,
            name: name,
        }
    }
}

impl<'a> shared_lib::SharedLibrary for SharedLibrary<'a> {
    fn each<F, C>(mut f: F)
        where F: FnMut(&Self) -> C,
              C: Into<shared_lib::IterationControl>
    {
        // Make sure we have exclusive access to dyld so that (hopefully) no one
        // else adds or removes shared libraries while we are iterating them.
        let _dyld_lock = DYLD_LOCK.lock();

        let count = unsafe {
            bindings::_dyld_image_count()
        };

        for image_idx in 0..count {
            let (header, slide, name) = unsafe {
                (bindings::_dyld_get_image_header(image_idx).as_ref(),
                 bindings::_dyld_get_image_vmaddr_slide(image_idx),
                 bindings::_dyld_get_image_name(image_idx))
            };
            if let Some(header) = header {
                assert!(slide != 0, "If we have a header pointer, slide should be valid");
                assert!(name != ptr::null(), "If we have a header pointer, name should be valid");

                let name = unsafe { CStr::from_ptr(name) };
                let shlib = SharedLibrary::new(header, slide, name);

                match f(&shlib).into() {
                    shared_lib::IterationControl::Break => break,
                    shared_lib::IterationControl::Continue => continue,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use macos;
    use shared_lib::{IterationControl, SharedLibrary};

    #[test]
    fn have_libdyld() {
        let mut found_dyld = false;
        macos::SharedLibrary::each(|shlib| {
            found_dyld |= shlib.name
                .to_bytes()
                .split(|c| *c == b'.' || *c == b'/')
                .find(|s| s == b"libdyld")
                .is_some();
        });
        assert!(found_dyld);
    }

    #[test]
    fn can_break() {
        let mut first_count = 0;
        macos::SharedLibrary::each(|_| {
            first_count += 1;
        });
        assert!(first_count > 2);

        let mut second_count = 0;
        macos::SharedLibrary::each(|_| {
            second_count += 1;

            if second_count == first_count - 1 {
                IterationControl::Break
            } else {
                IterationControl::Continue
            }
        });
        assert_eq!(second_count, first_count - 1);
    }
}
