//#![feature(maybe_uninit_ref)]
#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types, clippy::mut_from_ref, clippy::cast_ptr_alignment)]

use std::env;

use ese_parser_lib::{esent::*, ese_trait::*, ese_api::*};

use std::mem::{size_of, MaybeUninit};

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

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

fn dump_tables(dbpath: &str) {
    let mut jdb : EseAPI = EseDb::init();
    let v = jdb.load(dbpath);
    if v.is_some() {
        println!("Error: {:?}", v.unwrap());
        return ;
    }
    println!("jdb: {:?}", jdb);

    let tables = jdb.get_tables().unwrap();
    for t in tables
    {
        println!("table {}", &t);
        let cols = jdb.get_columns(&t).unwrap();
        for c in cols {
            println!("table {}, name {}, id {}, type {}, cbmax {}, cp {}", t, c.name, c.id, c.typ, c.cbmax, c.cp);
            let table_id = jdb.open_table(&t).unwrap();
            if jdb.move_row(table_id, JET_MoveFirst) {
                loop {
                    match c.typ {
                        JET_coltypBit => {
                            assert!(c.cbmax as usize == size_of::<i8>());
                            match jdb.get_column::<i8>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypUnsignedByte => {
                            assert!(c.cbmax as usize == size_of::<u8>());
                            match jdb.get_column::<u8>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypShort => {
                            assert!(c.cbmax as usize == size_of::<i16>());
                            match jdb.get_column::<i16>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypUnsignedShort => {
                            assert!(c.cbmax as usize == size_of::<u16>());
                            match jdb.get_column::<u16>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypLong => {
                            assert!(c.cbmax as usize == size_of::<i32>());
                            match jdb.get_column::<i32>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypUnsignedLong => {
                            assert!(c.cbmax as usize == size_of::<u32>());
                            match jdb.get_column::<u32>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypLongLong => {
                            assert!(c.cbmax as usize == size_of::<i64>());
                            match jdb.get_column::<i64>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypCurrency => {
                            assert!(c.cbmax as usize == size_of::<i64>());
                            match jdb.get_column::<i64>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypIEEESingle => {
                            assert!(c.cbmax as usize == size_of::<f32>());
                            match jdb.get_column::<f32>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypIEEEDouble => {
                            assert!(c.cbmax as usize == size_of::<f64>());
                            match jdb.get_column::<f64>(table_id, c.id).unwrap() {
                                Some(v) => println!("{}", v),
                                None => println!("NULL")
                            }
                        },
                        JET_coltypBinary => {
                            match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize) {
                                Ok(ov) => {
                                    match ov {
                                        Some(v) => {
                                            let s = v.iter().map(|c| format!("{:x?} ", c).to_string() ).collect::<String>();
                                            println!("{} ", s);
                                        },
                                        None => {
                                            println!("NULL");
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("get_column_dyn failed with error {}", e);
                                }
                            }
                        },
                        JET_coltypText => {
                            match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize) {
                                Ok(ov) => {
                                    match ov {
                                        Some(v) => {
                                            if c.cp == 1200 {
                                                let t = v.as_slice();
                                                unsafe {
                                                    let (_, v16, _) = t.align_to::<u16>();
                                                    let ws = OsString::from_wide(&v16);
                                                    let wss = ws.into_string().unwrap();
                                                    println!("{}", wss);
                                                }
                                            } else {
                                                let s = std::str::from_utf8(&v).unwrap();
                                                println!("{}", s);
                                            }
                                        },
                                        None => {
                                            println!("NULL");
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("get_column_dyn failed with error {}", e);
                                }
                            }
                        },
                        JET_coltypLongText => {
                            let v;
                            if c.cbmax == 0 {
                                v = jdb.get_column_dyn_varlen(table_id, c.id);
                            } else {
                                v = jdb.get_column_dyn(table_id, c.id, c.cbmax as usize);
                            }
                            match v {
                                Ok(ov) => {
                                    match ov {
                                        Some(v) => {
                                            if c.cp == 1200 {
                                                let t = v.as_slice();
                                                unsafe {
                                                    let (_, v16, _) = t.align_to::<u16>();
                                                    let ws = OsString::from_wide(&v16);
                                                    let wss = ws.into_string().unwrap();
                                                    if wss.len() > 70 {
                                                       println!("{} bytes: {}...", wss.len(), truncate(&wss, 70).to_string());
                                                    } else {
                                                        println!("{}", wss);
                                                    }
                                                }
                                            } else {
                                                let s = std::str::from_utf8(&v).unwrap();
                                                println!("{}", s);
                                            }
                                        },
                                        None => {
                                            println!("NULL");
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("get_column_dyn failed with error {}", e);
                                }
                            }
                        },
                        JET_coltypLongBinary => {
                            let v;
                            if c.cbmax == 0 {
                                v = jdb.get_column_dyn_varlen(table_id, c.id);
                            } else {
                                v = jdb.get_column_dyn(table_id, c.id, c.cbmax as usize);
                            }
                            match v {
                                Ok(ov) => {
                                    match ov {
                                        Some(v) => {
                                            let s = v.iter().map(|c| format!("{:x?} ", c).to_string() ).collect::<String>();
                                            if s.len() > 70 {
                                                println!("{} bytes: {}...", v.len(), truncate(&s, 70).to_string());
                                             } else {
                                                 println!("{}", s);
                                             }
                                        },
                                        None => {
                                            println!("NULL");
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("get_column_dyn failed with error {}", e);
                                }
                            }
                        },
                        JET_coltypGUID => {
                            match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize) {
                                Ok(ov) => {
                                    match ov {
                                        Some(v) => {
                                            unsafe {
                                                let mut vstr : Vec<u16> = Vec::new();
                                                vstr.resize(39, 0);
                                                let r = StringFromGUID2(v.as_ptr() as *const std::os::raw::c_void,
                                                    vstr.as_mut_ptr() as *const u16, vstr.len() as i32);
                                                if r > 0 {
                                                    let s = OsString::from_wide(&vstr).into_string().unwrap();
                                                    println!("{} ", s);
                                                }
                                            }
                                        },
                                        None => {
                                            println!("NULL");
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("get_column_dyn failed with error {}", e);
                                }
                            }
                        },
                        JET_coltypDateTime => {
                            assert!(c.cbmax as usize == size_of::<f64>());
                            match jdb.get_column::<f64>(table_id, c.id).unwrap() {
                                Some(v) => {
                                    let mut st = MaybeUninit::<SYSTEMTIME>::zeroed();
                                    unsafe {
                                        let r = VariantTimeToSystemTime(v, st.as_mut_ptr());
                                        if r == 1 {
                                            let s = st.assume_init();
                                            println!("{}.{}.{} {}:{}:{}", s.wDay, s.wMonth, s.wYear, s.wHour, s.wMinute, s.wSecond);
                                        } else {
                                            println!("VariantTimeToSystemTime failed");
                                        }
                                    }
                                },
                                None => println!("NULL")
                            }
                        },
                        _ => {
                            println!("Incorrect column type: {}, max is 19", c.typ);
                        }
                    }
                    if !jdb.move_row(table_id, JET_MoveNext) {
                        break;
                    }
                }
            }
            jdb.close_table(table_id);
        }
    }
}

#[test]
fn test_edb_table_all_values() {
    let mut jdb : EseAPI = EseDb::init();

    match jdb.load("C:\\temp\\ese\\test.db") {
        Some(e) => panic!("Error: {}", e),
        None => println!("jdb: {:?}", jdb)
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
    assert_eq!(jdb.get_column::<i16>(table_id, short.id).unwrap(), Some(32767));

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
            assert_eq!(s.wDayOfWeek, 0);
            assert_eq!(s.wDay, 13);
            assert_eq!(s.wMonth, 12);
            assert_eq!(s.wYear, 2020);
            assert_eq!(s.wHour, 11);
            assert_eq!(s.wMinute, 56);
            assert_eq!(s.wSecond, 32);
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
            let r = StringFromGUID2(gd.as_ptr() as *const c_void, vstr.as_mut_ptr() as *const u16, vstr.len() as i32);
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
        assert_eq!(long_binary.cbmax, 1024);

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

        let t = jdb.get_column_dyn(table_id, text.id, text.cbmax as usize).unwrap().unwrap();
        for i in 0..t.len() {
            assert_eq!(t[i], abc.as_bytes()[i %  abc.len()]);
        }
    }

    // LongText
    {
        let long_text = columns.iter().find(|x| x.name == "LongText" ).unwrap();
        assert_eq!(long_text.cbmax, 1024);
        assert_eq!(long_text.cp, 1200);

        let lt = jdb.get_column_dyn_varlen(table_id, long_text.id).unwrap().unwrap();
        let s = lt.as_slice();

        unsafe {
            let (_, v16, _) = s.align_to::<u16>();
            let ws = OsString::from_wide(&v16).into_string().unwrap();
            for i in 0..ws.len() {
                let l = ws.chars().nth(i).unwrap();
                let r = abc.as_bytes()[i %  abc.len()] as char;
                assert_eq!(l, r);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 0 {
        eprintln!("error: db path as input.");
        return;
    }
    let s = args.concat();

    dump_tables(&s);
}