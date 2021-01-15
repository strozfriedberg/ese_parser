extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!(r"cargo:rustc-link-lib=esent");
    println!(r"cargo:rustc-link-lib=ole32");
    println!(r"cargo:rustc-link-lib=oleaut32");
    println!(r"cargo:rustc-link-search=C:\Program Files (x86)\Windows Kits\10\Lib\10.0.17763.0\um\x64");

    println!("cargo:rerun-if-changed=src/esent.h");
    let bindings = bindgen::Builder::default()
        .header("src/esent.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // required while cross-build from Linux
        .clang_args(&[
            "-D__int64=long long",
            "-D_Pre_notnull_=",
            "-D_Out_writes_bytes_opt_(a)=",
            "-D_Out_cap_post_count_(a, b)="])
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/esent.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("esent.rs"))
        .expect("Couldn't write bindings!");
}
