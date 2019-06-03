fn main() {
    #[cfg(target_os = "macos")]
    {
        extern crate bindgen;

        use std::env;
        use std::path::PathBuf;

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
}
