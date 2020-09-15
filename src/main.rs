#![allow(non_camel_case_types,  clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;

use env_logger;
use std::process;

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

macro_rules! expect_eq {
    ($left:expr, $right:expr) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    error!(r#"expectation failed: `({} == {})`
  left: `{:?}`,
 right: `{:?}`"#, stringify!($left), stringify!($right), &*left_val, &*right_val)
                }
            }
        }
    });
    ($left:expr, $right:expr,) => ({
        expect_eq!($left, $right)
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    error!(r#"expectation failed: `({} == {})`
  left: `{:?}`,
 right: `{:?}`: {}"#, stringify!($left), stringify!($right), &*left_val, &*right_val,
                           format_args!($($arg)+))
                }
            }
        }
    });
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

#[macro_use] extern crate prettytable;
use prettytable::{Table};
extern crate hexdump;
use itertools::Itertools;
use  ese_parser::ese::db_file_header::{ esedb_file_header, esedb_file_signature };

pub fn dump_db_file_header(db_file_header: esedb_file_header) {
    let mut table = Table::new();

    macro_rules! add_field {
        ($fld: ident) => {table.add_row(row![stringify!($fld), db_file_header.$fld])}
    }
    macro_rules! add_dt_field {
        ($dt: ident) => {
            let s = format!("{:.4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}",
                            1900 + db_file_header.$dt[5] as u16, db_file_header.$dt[4], db_file_header.$dt[3],
                            db_file_header.$dt[2], db_file_header.$dt[1], db_file_header.$dt[0],);
            table.add_row(row![stringify!($dt), s]);
        }
    }
    macro_rules! add_64_field {
        ($fld: ident) => {
            let v = u64::from_be_bytes(db_file_header.$fld);
            table.add_row(row![stringify!($fld), format!("{}", v)]);
        }
    }
    macro_rules! add_hex_field {
        ($fld: ident) => {
            let mut s: String = "".to_string();
            hexdump::hexdump_iter(&db_file_header.$fld).foreach(|line| { s.push_str(&line); s.push_str("\n"); } );
            table.add_row(row![stringify!($fld), s]);
        }
    }

    add_field!(checksum);
    add_field!(signature);
    add_field!(format_version);
    add_field!(file_type);
    add_hex_field!(database_time);
    add_hex_field!(database_signature);
    add_field!(database_state);
    add_64_field!(consistent_postition);
    add_dt_field!(consistent_time);
    add_dt_field!(attach_time);
    add_64_field!(attach_postition);
    add_dt_field!(detach_time);
    add_64_field!(detach_postition);
    add_field!(unknown1);
    add_hex_field!(log_signature);
    add_hex_field!(previous_full_backup);
    add_hex_field!(previous_incremental_backup);
    add_hex_field!(current_full_backup);
    add_field!(shadowing_disabled);
    add_field!(last_object_identifier);
    add_field!(index_update_major_version);
    add_field!(index_update_minor_version);
    add_field!(index_update_build_number);
    add_field!(index_update_service_pack_number);
    add_field!(format_revision);
    add_field!(page_size);
    add_dt_field!(repair_time);
    add_hex_field!(unknown2);
    add_dt_field!(scrub_database_time);
    add_dt_field!(scrub_time);
    add_hex_field!(required_log);
    add_field!(upgrade_exchange5_format);
    add_field!(upgrade_free_pages);
    add_field!(upgrade_space_map_pages);
    add_hex_field!(current_shadow_volume_backup);
    add_field!(creation_format_version);
    add_field!(creation_format_revision);
    add_hex_field!(unknown3);
    add_field!(old_repair_count);
    add_field!(ecc_fix_success_count);
    add_dt_field!(ecc_fix_success_time);
    add_field!(old_ecc_fix_success_count);
    add_field!(ecc_fix_error_count);
    add_dt_field!(ecc_fix_error_time);
    add_field!(old_ecc_fix_error_count);
    add_field!(bad_checksum_error_count);
    add_dt_field!(bad_checksum_error_time);
    add_field!(old_bad_checksum_error_count);
    add_field!(committed_log);
    add_hex_field!(previous_shadow_volume_backup);
    add_hex_field!(previous_differential_backup);
    add_hex_field!(unknown4_1);
    add_hex_field!(unknown4_2);
    add_field!(nls_major_version);
    add_field!(nls_minor_version);
    add_hex_field!(unknown5_1);
    add_hex_field!(unknown5_2);
    add_hex_field!(unknown5_3);
    add_hex_field!(unknown5_4);
    add_hex_field!(unknown5_5);
    add_field!(unknown_flags);

    table.printstd();
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
    expect_eq!(stored_checksum, checksum, "wrong checksum");

    let backup_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(db_file_header.page_size as u64))
        .map_err(EseParserError::Io)?;

    if db_file_header.format_revision == 0 {
        db_file_header.format_revision = backup_file_header.format_revision;
    }

    expect_eq!(db_file_header.format_revision, backup_file_header.format_revision, "mismatch in format revision");

    if db_file_header.page_size == 0 {
        db_file_header.page_size = backup_file_header.page_size;
    }

    expect_eq!(db_file_header.page_size, backup_file_header.page_size, "mismatch in page size");
    expect_eq!(db_file_header.format_version, 0x620, "unsupported format version");

    dump_db_file_header(db_file_header);
    Ok(())
}

fn main() {
    env_logger::init();

    let config = Config::new().unwrap_or_else(|err| {  error!("Problem parsing arguments: {}", err);
                                                                   process::exit(1);
                                                                });
    info!("file '{}'", config.inp_file);

    if let Err(e) = run(config) {
        error!("Application error: {}", e);

        process::exit(1);
    }
}
