extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    cc::Build::new()
        .cpp(true)
        .files(&["cpp/decompress.cpp",
            "cpp/ms/checksum.cxx",
            "cpp/ms/checksum_amd64.cxx",
            "cpp/ms/checksum_avx.cxx",
            "cpp/ms/config.cxx",
            "cpp/ms/cprintf.cxx",
            "cpp/ms/currproc.cxx",
            "cpp/ms/dllentry.cxx",
            "cpp/ms/edbg.cxx",
            "cpp/ms/encrypt.cxx",
            "cpp/ms/error.cxx",
            "cpp/ms/event.cxx",
            "cpp/ms/hapublish.cxx",
            "cpp/ms/library.cxx",
            "cpp/ms/math.cxx",
            "cpp/ms/memory.cxx",
            "cpp/ms/norm.cxx",
            "cpp/ms/os.cxx",
            "cpp/ms/osblockcache.cxx",
            "cpp/ms/osdiag.cxx",
            "cpp/ms/osdisk.cxx",
            "cpp/ms/oseventtrace.cxx",
            "cpp/ms/osfile.cxx",
            "cpp/ms/osfs.cxx",
            "cpp/ms/ostimerqueue.cxx",
            "cpp/ms/perfmon.cxx",
            "cpp/ms/reftrace.cxx",
            "cpp/ms/string.cxx",
            "cpp/ms/sync.cxx",
            "cpp/ms/sysinfo.cxx",
            "cpp/ms/task.cxx",
            "cpp/ms/thread.cxx",
            "cpp/ms/time.cxx",
            "cpp/ms/trace.cxx",
            "cpp/ms/violated.cxx",
            "cpp/ms/_xpress/xdecode.c",
        ])
        .define("ESENT", None)
        .define("SORTPP_PASS", None)
        .define("DISABLE_ERR_CHECK", None)
        .define("RUST_LIBRARY", None)
        .compile("decompress");

    println!(r"cargo:rustc-link-lib=esent");
    println!(r"cargo:rustc-link-lib=ole32");
    println!(r"cargo:rustc-link-lib=oleaut32");
    println!(r"cargo:rustc-link-search=C:\Program Files (x86)\Windows Kits\10\Lib\10.0.17763.0\um\x64");

    println!("cargo:rerun-if-changed=cpp/esent.h");
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
