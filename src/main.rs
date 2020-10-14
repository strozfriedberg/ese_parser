#![feature(maybe_uninit_ref)]
#![allow(non_camel_case_types,  clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;
extern crate strum;

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
use ese_parser::util::config::{ Config };
use ese_parser::util::reader::{ EseParserError, read_struct };
use ese_parser::util::{ any_as_u8_slice, any_as_u32_slice };

pub fn load_db_file_header(config: &Config) -> Result<esedb_file_header, EseParserError> {
    let mut db_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(0))
                                        .map_err(EseParserError::Io)?;

    //debug!("db_file_header ({:X}): {:0X?}", mem::size_of::<esedb_file_header>(), db_file_header);

    assert_eq!(db_file_header.signature, esedb_file_signature, "bad file_header.signature");

    fn calc_crc32(file_header: &&mut esedb_file_header) -> u32 {
        let vec32: &[u32] = unsafe{ any_as_u32_slice(& file_header) };
        vec32.iter().skip(1).fold(0x89abcdef as u32, |crc, &val| crc ^ val )
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
fn get_database_file_info(config: &Config) -> Result<JET_DBINFOMISC4, EseParserError> { //TODO: check version
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

    let mut db_file_header = match load_db_file_header(&config) {
        Ok(x) => x ,
        Err(e) => {
            error!("Application error: {}", e);
            process::exit(1);
        }
    };

    //use ese_parser::util::dumper::{ dump_db_file_header };
    //dump_db_file_header(db_file_header);
    let mut db_info = get_database_file_info(&config).unwrap();
    //println!("{:?}", db_info);

    macro_rules! cmp {
        ($($dbf_fld: ident).+, $($info_fld: ident) . +) => {
            unsafe {
                assert_eq!(any_as_u8_slice(&mut &mut db_file_header.$($dbf_fld) . +), any_as_u8_slice(&mut &mut db_info.$($info_fld) . +));
            }
        };
    }

    cmp!(database_signature.random, signDb.ulRandom);
    cmp!(database_signature.computer_name, signDb.szComputerName);
    cmp!(database_signature.logtime_create, signDb.logtimeCreate);
    cmp!(consistent_postition, lgposConsistent);
    cmp!(consistent_time, logtimeConsistent);
    cmp!(attach_time, logtimeAttach);
    cmp!(attach_postition, lgposAttach);
    cmp!(detach_time, logtimeDetach);
    cmp!(detach_postition, lgposDetach);
    //cmp!(dbid, signLog);
    cmp!(previous_full_backup, bkinfoFullPrev);
    cmp!(previous_incremental_backup, bkinfoIncPrev);
    cmp!(current_full_backup, bkinfoFullCur);
    cmp!(shadowing_disabled, fShadowingDisabled);
    //cmp!(last_object_identifier, fUpgradeDb);
    cmp!(index_update_major_version, dwMajorVersion);
    cmp!(index_update_minor_version, dwMinorVersion);
    cmp!(index_update_build_number, dwBuildNumber);
    cmp!(index_update_service_pack_number, lSPNumber);
    cmp!(page_size, cbPageSize);
    cmp!(repair_count, ulRepairCount);
    cmp!(repair_time, logtimeRepair);

    cmp!(committed_log, genMaxRequired);

    cmp!(format_version, ulVersion);
    assert_eq!(db_file_header.database_state as ::std::os::raw::c_ulong, db_info.dbstate);
}
