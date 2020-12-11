#![feature(maybe_uninit_ref)]
#![allow(non_camel_case_types,  clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;
//#[macro_use] extern crate bitflags;
extern crate strum;

mod util;
mod ese;

use env_logger;

use std::process;

use crate::util::config::Config;
use crate::util::reader::{ load_db_file_header };
use crate::ese::jet;

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
    for i in 1..6 {
        let page = crate::ese::jet::PageHeader::new(&config, &io_handle, i);
        println!("Page {}:", i);
        util::dumper::dump_page_header(&page);
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
