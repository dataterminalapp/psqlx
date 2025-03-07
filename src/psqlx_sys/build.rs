use std::{env, path::PathBuf};

fn main() {
    let bindings = bindgen::Builder::default()
        .rustified_enum(".*")
        .prepend_enum_name(false)
        .translate_enum_integer_types(true)
        .header("wrapper.h")
        .allowlist_type("PsqlSettings")
        .allowlist_type("backslashResult")
        .allowlist_type("slash_option_type")
        .allowlist_item("slash_option_type")
        .allowlist_type("PQ.*")
        .allowlist_function("PQ.*")
        .allowlist_function("resetPQExpBuffer")
        .allowlist_function("appendPQExpBufferStr")
        .allowlist_function("psql_scan_slash_option")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Manually aliasing the enum in the generated bindings file
    // Add this manually after the bindings are generated.
    println!("cargo:rerun-if-changed=wrapper.h");
}
