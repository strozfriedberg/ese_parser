#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(target_os = "linux")]
include!("generated/esent.rs");

#[cfg(target_os = "windows")]
include!(concat!(env!("OUT_DIR"), "/esent.rs"));
