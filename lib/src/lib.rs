#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types, clippy::mut_from_ref, clippy::cast_ptr_alignment)]

mod parser;

pub mod esent;
pub mod ese_trait;
pub mod ese_api;
pub mod ese_parser;

use esent::*;
use ese_api::*;
use ese_trait::*;
use ese_parser::*;

use std::mem::MaybeUninit;
use std::ffi::OsString;
use std::os::windows::prelude::*;

extern "C" {
    pub fn StringFromGUID2(
        rguid: *const ::std::os::raw::c_void,
        lpsz: *const ::std::os::raw::c_ushort,
        cchMax: ::std::os::raw::c_int
    ) -> ::std::os::raw::c_int;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SYSTEMTIME {
    pub wYear: ::std::os::raw::c_ushort,
    pub wMonth: ::std::os::raw::c_ushort,
    pub wDayOfWeek: ::std::os::raw::c_ushort,
    pub wDay: ::std::os::raw::c_ushort,
    pub wHour: ::std::os::raw::c_ushort,
    pub wMinute: ::std::os::raw::c_ushort,
    pub wSecond: ::std::os::raw::c_ushort,
    pub wMilliseconds: ::std::os::raw::c_ushort,
}
extern "C" {
    pub fn VariantTimeToSystemTime(vtime: f64, lpSystemTime: *mut SYSTEMTIME) -> ::std::os::raw::c_int;
}

#[test]
fn test_edb_table_all_values() {
    //let mut jdb : EseAPI = EseDb::init();
    let mut jdb : EseParser = EseParser::init(5);

    match jdb.load("testdata\\test.edb") {
        Some(e) => panic!("Error: {}", e),
        None => println!("Loaded.")
    }

    let expected_tables = vec!["MSysObjects", "MSysObjectsShadow", "MSysObjids", "MSysLocales", "TestTable"];
    let tables = jdb.get_tables().unwrap();
    assert_eq!(tables.len(), expected_tables.len());
    for i in 0..tables.len() {
        assert_eq!(tables[i], expected_tables[i]);
    }

    let table = "TestTable";
    println!("table {}", table);

    let columns = jdb.get_columns(&table).unwrap();

    let table_id = jdb.open_table(&table).unwrap();
    assert_eq!(jdb.move_row(table_id, JET_MoveFirst), true);

    let bit = columns.iter().find(|x| x.name == "Bit" ).unwrap();
    assert_eq!(jdb.get_column::<i8>(table_id, bit.id).unwrap(), Some(0));

    let unsigned_byte = columns.iter().find(|x| x.name == "UnsignedByte" ).unwrap();
    assert_eq!(jdb.get_column::<u8>(table_id, unsigned_byte.id).unwrap(), Some(255));

    let short = columns.iter().find(|x| x.name == "Short" ).unwrap();
    assert_eq!(jdb.get_column::<i16>(table_id, short.id).unwrap(), Some(0));

    let long = columns.iter().find(|x| x.name == "Long" ).unwrap();
    assert_eq!(jdb.get_column::<i32>(table_id, long.id).unwrap(), Some(-2147483648));

    let currency = columns.iter().find(|x| x.name == "Currency" ).unwrap();
    assert_eq!(jdb.get_column::<i64>(table_id, currency.id).unwrap(), Some(350050)); // 35.0050

    let float = columns.iter().find(|x| x.name == "IEEESingle" ).unwrap();
    assert_eq!(jdb.get_column::<f32>(table_id, float.id).unwrap(), Some(3.141592));

    let double = columns.iter().find(|x| x.name == "IEEEDouble" ).unwrap();
    assert_eq!(jdb.get_column::<f64>(table_id, double.id).unwrap(), Some(3.141592653589));

    let unsigned_long = columns.iter().find(|x| x.name == "UnsignedLong" ).unwrap();
    assert_eq!(jdb.get_column::<u32>(table_id, unsigned_long.id).unwrap(), Some(4294967295));

    let long_long = columns.iter().find(|x| x.name == "LongLong" ).unwrap();
    assert_eq!(jdb.get_column::<i64>(table_id, long_long.id).unwrap(), Some(9223372036854775807));

    let unsigned_short = columns.iter().find(|x| x.name == "UnsignedShort" ).unwrap();
    assert_eq!(jdb.get_column::<u16>(table_id, unsigned_short.id).unwrap(), Some(65535));

    // DateTime
    {
        let date_time = columns.iter().find(|x| x.name == "DateTime" ).unwrap();
        let dt = jdb.get_column::<f64>(table_id, date_time.id).unwrap().unwrap();

        let mut st = MaybeUninit::<SYSTEMTIME>::zeroed();
        unsafe {
            let r = VariantTimeToSystemTime(dt, st.as_mut_ptr());
            assert_eq!(r, 1);
            let s = st.assume_init();
            assert_eq!(s.wDayOfWeek, 1);
            assert_eq!(s.wDay, 1);
            assert_eq!(s.wMonth, 3);
            assert_eq!(s.wYear, 2021);
            assert_eq!(s.wHour, 14);
            assert_eq!(s.wMinute, 4);
            assert_eq!(s.wSecond, 25);
            assert_eq!(s.wMilliseconds, 0);
        }
    }

    // GUID
    {
        let guid = columns.iter().find(|x| x.name == "GUID" ).unwrap();
        let gd = jdb.get_column_dyn(table_id, guid.id, guid.cbmax as usize).unwrap().unwrap();
        unsafe {
            let mut vstr : Vec<u16> = Vec::new();
            vstr.resize(39, 0);
            let r = StringFromGUID2(gd.as_ptr() as *const ::std::os::raw::c_void, vstr.as_mut_ptr() as *const u16, vstr.len() as i32);
            assert_ne!(r, 0);
            let s = OsString::from_wide(&vstr).into_string().unwrap();
            assert_eq!(s, "{4D36E96E-E325-11CE-BFC1-08002BE10318}\u{0}");
        }
    }

    // Binary
    {
        let binary = columns.iter().find(|x| x.name == "Binary" ).unwrap();
        assert_eq!(binary.cbmax, 255);

        let b = jdb.get_column_dyn(table_id, binary.id, binary.cbmax as usize).unwrap().unwrap();
        for i in 0..b.len() {
            assert_eq!(b[i], (i % 255) as u8);
        }
    }

    // LongBinary
    {
        let long_binary = columns.iter().find(|x| x.name == "LongBinary" ).unwrap();
        assert_eq!(long_binary.cbmax, 8600);

        let b = jdb.get_column_dyn_varlen(table_id, long_binary.id).unwrap().unwrap();
        for i in 0..b.len() {
            assert_eq!(b[i], (i % 255) as u8);
        }
    }

    let abc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890";

    // Text
    {
        let text = columns.iter().find(|x| x.name == "Text" ).unwrap();
        assert_eq!(text.cbmax, 255);
        assert_eq!(text.cp, 1252);

        let str = jdb.get_column_str(table_id, text.id, text.cbmax as u32).unwrap().unwrap();
        let t = jdb.get_column_dyn(table_id, text.id, text.cbmax as usize).unwrap().unwrap();
        for i in 0..t.len() {
            let expected_char = abc.as_bytes()[i % abc.len()];
            assert_eq!(t[i], expected_char);
            assert_eq!(str.as_bytes()[i], expected_char);
        }

        // Multi-value test
        let v = jdb.get_column_dyn_mv(table_id, text.id, 2).unwrap().unwrap();
        let h = "Hello".to_string();
        assert_eq!(v.len()-2, h.len());
        for i in 0..v.len()-2 {
            assert_eq!(v[i], h.as_bytes()[i]);
        }
    }

    // LongText
    {
        let long_text = columns.iter().find(|x| x.name == "LongText" ).unwrap();
        assert_eq!(long_text.cbmax, 8600);
        assert_eq!(long_text.cp, 1200);

        let lt = jdb.get_column_dyn_varlen(table_id, long_text.id).unwrap().unwrap();
        let s = lt.as_slice();

        unsafe {
            let (_, v16, _) = s.align_to::<u16>();
            let ws = OsString::from_wide(&v16).into_string().unwrap();
            for i in 0..ws.len() {
                let l = ws.chars().nth(i).unwrap();
                let r = abc.as_bytes()[i % abc.len()] as char;
                assert_eq!(l, r);
            }
        }
    }

    jdb.close_table(table_id);
}
