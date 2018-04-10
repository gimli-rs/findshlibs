extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    if cfg!(target_os = "linux") {
        generate_linux_bindings();
    } else if cfg!(target_os = "macos") {
        generate_macos_bindings();
    } else {
        panic!("`findshlibs` does not support the target OS :(");
    }
}

fn generate_linux_bindings() {
    let bindings = bindgen::Builder::default()
        .header("./src/linux/bindings.h")
        .whitelist_function("dl_iterate_phdr")
        .whitelist_type(r#"Elf\d*.*"#)
        .whitelist_var("PT_.*")
        .generate()
        .expect("Should generate linux FFI bindings OK");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("linux_bindings.rs"))
        .expect("Should write linux_bindings.rs OK");
}

fn generate_macos_bindings() {
    let bindings = bindgen::Builder::default()
        .header("./src/macos/bindings.h")
        .whitelist_function("_dyld_.*")
        .whitelist_type("mach_header.*")
        .whitelist_type("load_command.*")
        .whitelist_type("uuid_command.*")
        .whitelist_type("segment_command.*")
        .whitelist_var("MH_MAGIC.*")
        .whitelist_var("LC_SEGMENT.*")
        .whitelist_var("LC_UUID.*")
        .generate()
        .expect("Should generate macOS FFI bindings OK");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("macos_bindings.rs"))
        .expect("Should write macos_bindings.rs OK");
}
