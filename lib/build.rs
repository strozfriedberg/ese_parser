extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!(r"cargo:rustc-link-lib=esent");
    println!(r"cargo:rustc-link-lib=Ole32");
    println!(r"cargo:rustc-link-lib=OleAut32");
    println!(r"cargo:rustc-link-search=C:\Program Files (x86)\Windows Kits\10\Lib\10.0.17763.0\um\x64");

    println!("cargo:rerun-if-changed=src/esent.h");
    let bindings = bindgen::Builder::default()
        .header("src/esent.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/esent.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("esent.rs"))
        .expect("Couldn't write bindings!");
}
