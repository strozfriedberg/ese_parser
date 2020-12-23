#![feature(maybe_uninit_ref)]
#![allow(non_camel_case_types,  clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;
//#[macro_use] extern crate bitflags;
#[macro_use] extern crate bitfield;
extern crate strum;

mod util;
mod ese;

use env_logger;

use std::process;

use crate::util::config::Config;
use crate::util::reader::{ load_db_file_header };
use crate::ese::{jet, ese_db};
use std::process::Command;
use std::io::{Cursor, BufRead};

/*
use crate::ese::esent::{JET_errSuccess, JET_DBINFOMISC4, JET_DbInfoMisc, JetGetDatabaseFileInfoA};

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
*/

fn main() {
    env_logger::init();

    let config = Config::new()
                        .unwrap_or_else(|err| { error!("Problem parsing arguments: {}", err);
                                                         process::exit(1);
                                                       });
    info!("file '{}'", config.inp_file.display());

    let db_file_header = match load_db_file_header(&config) {
        Ok(x) => x ,
        Err(e) => {
            error!("Application error: {}", e);
            process::exit(1);
        }
    };

    let io_handle = jet::IoHandle::new(&db_file_header);
    let pages = [jet::FixedPageNumber::Database, jet::FixedPageNumber::Catalog];

    for i in pages.iter() {
        let db_page = jet::DbPage::new(&config, &io_handle, *i as u32);
        println!("Page {:?}:", i);
        util::dumper::dump_page_header(&db_page.page_header);

        for pg_tag in &db_page.page_tags {
            let size = pg_tag.size();
            let offset = pg_tag.offset();
            println!("offset: {:#x} ({}), size: {:#x} ({})", offset, offset, size, size);

            {
                use nom::{
                    error, error::{VerboseError},
                    bytes, bytes::complete::{tag, is_not},
                    sequence::{terminated, separated_pair},
                };
                type Res<T, U> = nom::IResult<T, U, VerboseError<T>>;

                fn find_tag(line: &str) -> Res<&str, &str> {
                    error::context(
                        "find_tag",
                        tag("TAG ")
                    ) (line)
                }

                fn find_data(line: &str) -> Res<&str, &str> {
                    error::context(
                        "find_data",
                        bytes::complete::is_not("c")
                    ) (line)
                }

                fn extract_data(line: &str) -> Res<&str, &str> {
                    match find_data(line) {
                        Ok((data, _)) => terminated(is_not(" "), tag(" "))(data),
                        Err(e) => Err(e)
                    }
                }

                fn get_hex_value(data: &str, val_name: &str) -> u32 {
                    fn find<'a>(what: &'a str, line: &'a str) -> Res<&'a str, &'a str> {
                        tag(what)(line)
                    }
                    match find (val_name, data) {
                        Ok((val, _)) => u32::from_str_radix(val, 16).unwrap(),
                        _ => 0
                    }
                }

                #[derive(Debug)]
                struct Tag {
                    pub offset: u32,
                    pub flags: u32,
                    pub size: u32,
                }
                fn fill_tag(data: &str) -> Tag {
                    fn split(line: &str) -> Res<&str, (&str, &str)> {
                        separated_pair(is_not(","), tag(","), is_not(","))(line)
                    }
                    match split(data) {
                        Ok((_, (cb, ib))) => {
                            let size = get_hex_value(cb, "cb:0x");
                            let offset = get_hex_value(ib, "ib:0x");
                            Tag{ size: size, offset: offset, flags: 0}
                        },
                        Err(e) => panic!("{:?}", e)
                    }
                }

                let output = Command::new("esentutl")
                    .args(&["/ms", config.inp_file.to_str().unwrap(), &format!("/p{}",*i as u32)])
                    .output()
                    .expect("failed to execute process");
                let mut file = Cursor::new(output.stdout);
                for line in file.lines() {
                    match find_tag(&line.unwrap()[..]) {
                        Ok((res, _)) =>  {
                                let (rem, data) = extract_data(&res).unwrap();
                                //println!("rem: '{}', data: '{}'", rem, data);
                                let tag = fill_tag(data);
                                println!("{:?}", tag);
                            },
                        _ => {}
                        }
                    }

            }
        }
    }

/*
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
*/
}
