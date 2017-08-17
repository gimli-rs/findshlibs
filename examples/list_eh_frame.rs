extern crate findshlibs;
use findshlibs::{NamedMemoryRange, SharedLibrary, TargetSharedLibrary};

fn main() {
    TargetSharedLibrary::each(|shlib| {
        println!("{}", shlib.name().to_string_lossy());

        if let Some(eh_frame) = shlib.eh_frame() {
            println!(
                "    .eh_frame @ {}",
                eh_frame.actual_virtual_memory_address(shlib),
            );
        } else {
            println!("    (no .eh_frame)");
        }

        if let Some(eh_frame_hdr) = shlib.eh_frame_hdr() {
            println!(
                "    .eh_frame_hdr @ {}",
                eh_frame_hdr.actual_virtual_memory_address(shlib),
            );
        } else {
            println!("    (no .eh_frame_hdr)");
        }
    });
}
