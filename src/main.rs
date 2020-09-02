#![allow(non_camel_case_types)]
#![feature(maybe_uninit_ref)]

use std::error::Error;
use std::process;
// use std::fs;
// use std::fs::File;
// use std::io::Read;

use libc;
pub type uint32_t = libc::c_uint;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct libesedb_file_header {
    pub file_type: uint32_t,
    pub creation_format_version: uint32_t,
    pub creation_format_revision: uint32_t,
    pub format_revision: uint32_t,
    pub format_version: uint32_t,
    pub page_size: uint32_t,
}

extern crate clap;
use clap::{Arg, App};

pub struct Config {
    pub inp_file: String,
    pub report_file: String,
}

impl Config {
    pub fn new() -> Result<Config, &'static str> {
        let matches = App::new("ESE DB dump")
            .version("0.1.0")
            .arg(Arg::with_name("in")
                .short("i")
                .long("input")
                .takes_value(true)
                .required(true)
                .help("Path to ESE db file"))
            .arg(Arg::with_name("out")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Path to output report"))
            .get_matches();

        let inp_file = matches.value_of("in").unwrap().to_owned();
        println!(" inp_file: {}", inp_file);

        let report_file = matches.value_of("out").to_owned();
        match report_file {
            Some(s) => s,
            _ => &""
        };

        Ok(Config { inp_file, report_file : report_file.unwrap().to_string() })
    }
}

//https://stackoverflow.com/questions/38334994/how-to-read-a-c-struct-from-a-binary-file-in-rust
use std::io::{self, BufReader, Read};
use std::fs::File;
use std::mem::MaybeUninit;
use std::slice;

fn read_struct<T, R: Read>(mut read: R) -> io::Result<T> {
    let num_bytes = ::std::mem::size_of::<T>();
    unsafe {
        let mut s = MaybeUninit::<T>::zeroed();
        let mut buffer = slice::from_raw_parts_mut(s.get_ref() as *mut T as *mut u8, num_bytes);
        match read.read_exact(buffer) {
            Ok(()) => Ok(unsafe { s.get_ref() as T }),
            Err(e) => {
                ::std::mem::forget(s);
                Err(e)
            }
        }
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let mut reader = BufReader::new(File::open(config.inp_file)?);
    let file_header = read_struct::<libesedb_file_header>(reader);

    println!("{:0x?}", file_header);

    Ok(())
}

fn main() {
    let config = Config::new().unwrap_or_else(|err| {  println!("Problem parsing arguments: {}", err);
                                                                   process::exit(1);
                                                                });
    println!("file '{}'", config.inp_file);

    if let Err(e) = run(config) {
        println!("Application error: {}", e);

        process::exit(1);
    }
}
