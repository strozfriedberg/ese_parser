#![allow(non_camel_case_types, non_upper_case_globals, clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;

use env_logger;

use std::process;

pub type uint8_t  = libc::c_uchar;
pub type uint32_t = libc::c_uint;
pub type uint64_t = libc::c_long;

static  esedb_file_signature: uint32_t = 0x89abcdef;

pub type esedb_file_header_t = esedb_file_header;
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct esedb_file_header {
    pub checksum: uint32_t,
    pub signature: uint32_t,
    pub format_version: uint32_t,
    pub file_type: uint32_t,
    pub database_time: [uint8_t; 8],
    pub database_signature: [uint8_t; 28],
    pub database_state: uint32_t,
    pub consistent_postition: [uint8_t; 8],
    pub consistent_time: [uint8_t; 8],
    pub attach_time: [uint8_t; 8],
    pub attach_postition: [uint8_t; 8],
    pub detach_time: [uint8_t; 8],
    pub detach_postition: [uint8_t; 8],
    pub unknown1: uint32_t,
    pub log_signature: [uint8_t; 28],
    pub previous_full_backup: [uint8_t; 24],
    pub previous_incremental_backup: [uint8_t; 24],
    pub current_full_backup: [uint8_t; 24],
    pub shadowing_disabled: uint32_t,
    pub last_object_identifier: uint32_t,
    pub index_update_major_version: uint32_t,
    pub index_update_minor_version: uint32_t,
    pub index_update_build_number: uint32_t,
    pub index_update_service_pack_number: uint32_t,
    pub format_revision: uint32_t,
    pub page_size: uint32_t,
    pub repair_count: uint32_t,
    pub repair_time: [uint8_t; 8],
    pub unknown2: [uint8_t; 28],
    pub scrub_database_time: [uint8_t; 8],
    pub scrub_time: [uint8_t; 8],
    pub required_log: [uint8_t; 8],
    pub upgrade_exchange5_format: uint32_t,
    pub upgrade_free_pages: uint32_t,
    pub upgrade_space_map_pages: uint32_t,
    pub current_shadow_volume_backup: [uint8_t; 24],
    pub creation_format_version: uint32_t,
    pub creation_format_revision: uint32_t,
    pub unknown3: [uint8_t; 16],
    pub old_repair_count: uint32_t,
    pub ecc_fix_success_count: uint32_t,
    pub ecc_fix_success_time: [uint8_t; 8],
    pub old_ecc_fix_success_count: uint32_t,
    pub ecc_fix_error_count: uint32_t,
    pub ecc_fix_error_time: [uint8_t; 8],
    pub old_ecc_fix_error_count: uint32_t,
    pub bad_checksum_error_count: uint32_t,
    pub bad_checksum_error_time: [uint8_t; 8],
    pub old_bad_checksum_error_count: uint32_t,
    pub committed_log: uint32_t,
    pub previous_shadow_volume_backup: [uint8_t; 24],
    pub previous_differential_backup: [uint8_t; 24],
    pub unknown4_1: [uint8_t; 20],
    pub unknown4_2: [uint8_t; 40 - 20],
    pub nls_major_version: uint32_t,
    pub nls_minor_version: uint32_t,
    pub unknown5_1: [uint8_t; 32],
    pub unknown5_2: [uint8_t; 32],
    pub unknown5_3: [uint8_t; 32],
    pub unknown5_4: [uint8_t; 32],
    pub unknown5_5: [uint8_t; 148 - 4 * 32],
    pub unknown_flags: uint32_t,
    pub unknown_val: uint32_t,
}

use simple_error::SimpleError;
use std::fmt;

#[derive(Debug)]
pub enum EseParserError {
    Io(io::Error),
    Parse(SimpleError),
}
impl fmt::Display for EseParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EseParserError::Io(ref err) => write!(f, "IO error: {}", err),
            EseParserError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
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
        debug!(" inp_file: {}", inp_file);

        let report_file = matches.value_of("out").to_owned();
        match report_file {
            Some(s) => s,
            _ => &""
        };

        Ok(Config { inp_file, report_file : "".to_string()/*report_file.unwrap().to_string()*/ })
    }
}

//https://stackoverflow.com/questions/38334994/how-to-read-a-c-struct-from-a-binary-file-in-rust
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::mem::{self};
use std::slice;

fn read_struct<T, P: AsRef<Path>>(path: P, file_offset: SeekFrom) -> io::Result<T> {
    let path = path.as_ref();
    let struct_size = mem::size_of::<T>();
    let mut reader = BufReader::new(File::open(path)?);
    reader.seek(file_offset)?;
    let mut r : T = unsafe { mem::zeroed() };
    unsafe {
        let buffer = slice::from_raw_parts_mut(&mut r as *mut _ as *mut u8, struct_size);
        reader.read_exact(buffer)?;
    }
    Ok(r)
}

pub fn run(config: Config) -> Result<(), EseParserError> {
    let mut db_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(0))
                                        .map_err(EseParserError::Io)?;

    debug!("db_file_header ({:X}): {:0X?}", mem::size_of::<esedb_file_header>(), db_file_header);

    assert_eq!(db_file_header.signature, esedb_file_signature, "bad file_header.signature");

    fn calc_crc32(file_header: &&mut esedb_file_header) -> u32 {
        unsafe fn any_as_u8_slice<'a, T: Sized>(p: &'a &mut T) -> &'a mut [u8] {
            slice::from_raw_parts_mut((*p as *const T) as *mut u8, mem::size_of::<T>())
        }
        let vec8: &mut [u8] = unsafe{ any_as_u8_slice(& file_header) };
        let vec32 = unsafe {
            let length = (vec8.len() - 4) / mem::size_of::<u32>();
            let capacity = vec8.len() - 4;
            let ptr = vec8.as_mut_ptr().add(4) as *mut u32;

            Vec::from_raw_parts(ptr, length, capacity)
        };

        let mut crc32 = 0x89abcdef;
        for &val in &vec32 {
            crc32 ^= val;
        }

        mem::forget(vec32);
        crc32
    }

    let stored_checksum = db_file_header.checksum;
    let checksum = calc_crc32(&&mut db_file_header);
    assert_eq!(stored_checksum, checksum, "wrong checksum");

    let backup_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(db_file_header.page_size as u64))
        .map_err(EseParserError::Io)?;
    if db_file_header.format_revision == 0 {
        db_file_header.format_revision = backup_file_header.format_revision;
    }
    else if db_file_header.format_revision != backup_file_header.format_revision {
        warn!("mismatch in format revision: {:#X} and backup: {:#X}", db_file_header.format_revision, backup_file_header.format_revision);
    }
    
    if db_file_header.page_size == 0 {
        db_file_header.page_size = backup_file_header.page_size;
    }
    else if db_file_header.page_size != backup_file_header.page_size {
        warn!("mismatch in page size: {:#X} and backup: {:#X}", db_file_header.page_size, backup_file_header.page_size );
        //db_file_header.page_size = (uint32_t) file_offset;
    }

    Ok(())
}

fn main() {
    //env_logger::init();
    let _ = env_logger::builder().is_test(true).try_init();

    let config = Config::new().unwrap_or_else(|err| {  error!("Problem parsing arguments: {}", err);
                                                                   process::exit(1);
                                                                });
    info!("file '{}'", config.inp_file);

    if let Err(e) = run(config) {
        error!("Application error: {}", e);

        process::exit(1);
    }
}
