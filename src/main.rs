#![allow(non_camel_case_types)]

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

        Ok(Config { inp_file, report_file : "".to_string()/*report_file.unwrap().to_string()*/ })
    }
}

//https://stackoverflow.com/questions/38334994/how-to-read-a-c-struct-from-a-binary-file-in-rust
use std::io::{self, BufReader, Read};
use std::fs::File;
use std::path::Path;
use std::mem;
use std::slice;

fn read_struct<T, P: AsRef<Path>>(path: P) -> io::Result<T> {
    let path = path.as_ref();
    let struct_size = ::std::mem::size_of::<T>();
    let mut reader = BufReader::new(File::open(path)?);
    let mut r : T = unsafe { mem::zeroed() };
    unsafe {
        let buffer = slice::from_raw_parts_mut(&mut r as *mut _ as *mut u8, struct_size);
        reader.read_exact(buffer)?;
    }
    Ok(r)
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let file_header = read_struct::<libesedb_file_header, _>(config.inp_file);

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
