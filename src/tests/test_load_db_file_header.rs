//test_load_db_file_header.rs

use std::io::{self, Write};
use std::process::Command;
use std::str;
use regex::Regex;

use crate::tests::lib::*;

#[test]
fn test_load_db_file_header() {
    let entourage = Entourage::new();
    let db_file_header = entourage.db_file_header;

    let output = Command::new("esentutl")
        .args(&["/mh", entourage.config.inp_file.to_str().unwrap()])
        .output()
        .expect("failed to execute process");
    let text = str::from_utf8(&output.stdout).unwrap();

    macro_rules! check_field {
        ( $fld: expr, $closure:tt ) => {
            let val = get_field($fld, text).unwrap().trim();
            $closure(val);
        }
    };

    fn get_field<'a>(fld: &'a str, input: &'a str) -> Option<&'a str> {
        let s = format!("{}:(?P<value>.*)", fld);
        let re = Regex::new(&s).unwrap();

        re.captures(input).and_then(|cap| {
            cap.name("value").map(|login| login.as_str())
        })
    }

    fn u32_from_opt(str: &str) -> u32 {
        if let Ok(val) = u32::from_str_radix(str.trim_start_matches("0x"), 16) {
            return val;
        }
        println!("could not parse hex {}", str);
        io::stdout().flush().unwrap();
        //panic!("could not parse hex {}", str);
        return 0;
    }

    use crate::ese::jet::DbState;
    fn db_state_to_string(state: DbState) -> & 'static str {
        match state {
            DbState::impossible => "impossible",
            DbState::JustCreated => "Just Created",
            DbState::DirtyShutdown => "Dirty Shutdown",
            DbState::CleanShutdown => "Clean Shutdown",
            DbState::BeingConverted => "Being Converted",
            DbState::ForceDetach => "Force Detach",
        }
    }
    check_field!("Checksum", (|val: &str| assert_eq!(u32_from_opt(val), db_file_header.checksum) ));
    check_field!("Format ulMagic", (|val: &str| assert_eq!(u32_from_opt(val), db_file_header.signature) ));
    check_field!("State", (|val: &str| assert_eq!(val, db_state_to_string(db_file_header.database_state) )));
    check_field!("Last Consistent", (|val: &str| {
        let dt = &db_file_header.consistent_time;
        let s = format!("{:0>2}/{:0>2}/{:.4} {:0>2}:{:0>2}:{:0>2}",
                dt.month, dt.day, dt.year as u16 + 1900,
                dt.hours, dt.minutes, dt.seconds,);

        let val: Vec<_> = val.split(")  ").collect();

        println!("s '{}', val '{}'", s, val[1]);
        assert!(val[1].starts_with(&s))
    }));
/*
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