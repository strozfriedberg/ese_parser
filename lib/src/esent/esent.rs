#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// generated in build.rs by bindgen from esent.h
#[cfg(all(feature = "nt_comparison", target_os = "windows"))]
include!(concat!(env!("OUT_DIR"), "/esent.rs"));
