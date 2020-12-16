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

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 0 {
        eprintln!("error: db path as input.");
        return;
    }
    let s = args.concat();

    dump_tables(&s);
}