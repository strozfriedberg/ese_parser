#![feature(maybe_uninit_ref)]
#![allow(non_camel_case_types,  clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;
extern crate strum;

use std::mem;
use std::slice;
use std::io::SeekFrom;

use env_logger;

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

use ese_parser::ese::db_file_header::{ esedb_file_header, esedb_file_signature };
use ese_parser::util::dumper::{ dump_db_file_header };
use ese_parser::util::config::{ Config };
use ese_parser::util::reader::{ EseParserError, read_struct };

pub fn load_db_file_header(config: &Config) -> Result<esedb_file_header, EseParserError> {
    let mut db_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(0))
                                        .map_err(EseParserError::Io)?;

    //debug!("db_file_header ({:X}): {:0X?}", mem::size_of::<esedb_file_header>(), db_file_header);

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

    Ok(db_file_header)
}

//use std::str;
use std::process;
use std::ffi::{CString};
use std::os::raw::{c_void, c_ulong};
use std::mem::{size_of, MaybeUninit};
use simple_error::SimpleError;
use ese_parser::ese::esent::{JET_errSuccess, JET_DBINFOMISC4, JET_DbInfoMisc, JetGetDatabaseFileInfoA};

#[link(name = "esent")]
fn get_database_file_info(config: &Config) -> Result<JET_DBINFOMISC4, EseParserError> {
    let filename = CString::new(config.inp_file.as_bytes()).unwrap();
    let db_info = MaybeUninit::<JET_DBINFOMISC4>::zeroed();
    let res_size = size_of::<JET_DBINFOMISC4>() as c_ulong;

    unsafe {
        let ptr: *mut c_void = db_info.as_ptr() as *mut c_void;
        let res = JetGetDatabaseFileInfoA(filename.as_ptr(), ptr, res_size, JET_DbInfoMisc) as u32;

        if JET_errSuccess == res {
            Ok(*(db_info.get_ref()))
        }
        else {
            Err(EseParserError::Parse(SimpleError::new(format!("error={}", res))))
        }
    }
}

fn main() {
    env_logger::init();

    let config = Config::new().unwrap_or_else(|err| {  error!("Problem parsing arguments: {}", err);
                                                                   process::exit(1);
                                                                });
    info!("file '{}'", config.inp_file);

    let db_file_header = match load_db_file_header(&config) {
        Ok(x) => x ,
        Err(e) => {
            error!("Application error: {}", e);
            process::exit(1);
        }
    };

    dump_db_file_header(db_file_header);
    let db_info = get_database_file_info(&config).unwrap();
    println!("{:?}", db_info);
    assert_eq!(db_file_header.format_version, db_info.ulVersion);
    /*
    check_field!("Checksum", (|val: &str| assert_eq!(u32_from_opt(val), db_file_header.checksum) ));
    check_field!("Format ulMagic", (|val: &str| assert_eq!(u32_from_opt(val), db_file_header.signature) ));
    //check_field!("Format ulVersion", (|val: &str| assert_eq!(u32_from_opt(val), db_file_header.format_version) ));
    check_field!("Last Consistent", (|val: &str| {
        let dt = &db_file_header.consistent_time;
        let s = format!("{:0>2}/{:0>2}/{:.4} {:0>2}:{:0>2}:{:0>2}",
                dt[4], dt[3], 1900 + dt[5] as u16,
                dt[2], dt[1], dt[0],);

        println!("s {}, val {}", s, val);
        assert!(val.starts_with(&s))
    }));
     */
}
