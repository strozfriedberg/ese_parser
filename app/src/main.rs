#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types, clippy::mut_from_ref, clippy::cast_ptr_alignment)]

use std::env;
use ese_parser_lib::{esent::*, ese_trait::*, ese_parser::*};
use std::mem::size_of;
use simple_error::SimpleError;
use widestring::U16String;

const CACHE_SIZE_ENTRIES : usize = 10;

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

fn get_column<T>(jdb: &Box<dyn EseDb>, table: u64, column: u32) -> Result<Option<T>, SimpleError> {
    let size = size_of::<T>();
    let mut dst = std::mem::MaybeUninit::<T>::zeroed();

    let vo = jdb.get_column_dyn(table, column, size)?;

    unsafe {
        if let Some(v) = vo {
            std::ptr::copy_nonoverlapping(
                v.as_ptr(),
                dst.as_mut_ptr() as *mut u8,
                size);
            return Ok(Some(dst.assume_init()));
        }
        return Ok(None);
    }
}

fn get_column_val(jdb: &Box<dyn EseDb>, table_id: u64, c: &ColumnInfo) -> Result<String, SimpleError> {
    let mut val = String::new();
    match c.typ {
        JET_coltypBit => {
            assert!(c.cbmax as usize == size_of::<i8>());
            match get_column::<i8>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypUnsignedByte => {
            assert!(c.cbmax as usize == size_of::<u8>());
            match get_column::<u8>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypShort => {
            assert!(c.cbmax as usize == size_of::<i16>());
            match get_column::<i16>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypUnsignedShort => {
            assert!(c.cbmax as usize == size_of::<u16>());
            match get_column::<u16>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypLong => {
            assert!(c.cbmax as usize == size_of::<i32>());
            match get_column::<i32>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypUnsignedLong => {
            assert!(c.cbmax as usize == size_of::<u32>());
            match get_column::<u32>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypLongLong => {
            assert!(c.cbmax as usize == size_of::<i64>());
            match get_column::<i64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypUnsignedLongLong => {
            assert!(c.cbmax as usize == size_of::<u64>());
            match get_column::<u64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypCurrency => {
            assert!(c.cbmax as usize == size_of::<i64>());
            match get_column::<i64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypIEEESingle => {
            assert!(c.cbmax as usize == size_of::<f32>());
            match get_column::<f32>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypIEEEDouble => {
            assert!(c.cbmax as usize == size_of::<f64>());
            match get_column::<f64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = format!(" ")
            }
        },
        JET_coltypBinary => {
            match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize)? {
                Some(v) => {
                    let s = v.iter().map(|c| format!("{:x?} ", c).to_string() ).collect::<String>();
                    val = format!("{} ", s);
                },
                None => {
                    val = format!(" ");
                }
            }
        },
        JET_coltypText => {
            match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize)? {
                Some(v) => {
                    if c.cp == 1200 {
                        let t = v.as_slice();
                        unsafe {
                            let (_, v16, _) = t.align_to::<u16>();
                            let U16Str = U16String::from_ptr(v16.as_ptr(), v16.len());
                            let ws = U16Str.to_string_lossy();
                            val = format!("{}", ws);
                        }
                    } else {
                        let s = std::str::from_utf8(&v).unwrap();
                        val = format!("{}", s);
                    }
                },
                None => {
                    val = format!(" ");
                }
            }
        },
        JET_coltypLongText => {
            let v;
            if c.cbmax == 0 {
                v = jdb.get_column_dyn_varlen(table_id, c.id)?;
            } else {
                v = jdb.get_column_dyn(table_id, c.id, c.cbmax as usize)?;
            }
            match v {
                Some(v) => {
                    if c.cp == 1200 {
                        let t = v.as_slice();
                        unsafe {
                            let (_, v16, _) = t.align_to::<u16>();
                            let U16Str = U16String::from_ptr(v16.as_ptr(), v16.len());
                            let ws = U16Str.to_string_lossy();
                            if ws.len() > 32 {
                                val = format!("{:4} bytes: {}...", ws.len(), truncate(&ws, 32).to_string());
                            } else {
                                val = format!("{}", ws);
                            }
                        }
                    } else {
                        let s = std::str::from_utf8(&v).unwrap();
                        val = format!("{}", s);
                    }
                },
                None => {
                    val = format!(" ");
                }
            }
        },
        JET_coltypLongBinary => {
            let v;
            if c.cbmax == 0 {
                v = jdb.get_column_dyn_varlen(table_id, c.id)?;
            } else {
                v = jdb.get_column_dyn(table_id, c.id, c.cbmax as usize)?;
            }
            match v {
                Some(mut v) => {
                    let orig_size = v.len();
                    v.truncate(16);
                    let s = v.iter().map(|c| format!("{:02x} ", c).to_string() ).collect::<String>();
                    val = format!("{:4} bytes: {}...", orig_size, s);
                },
                None => {
                    val = format!(" ");
                }
            }
        },
        JET_coltypGUID => {
            match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize)? {
                Some(v) => {
                    // {CD2C96BD-DCA8-47CB-B829-8F1AE4E2E686}
                    val = format!("{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
                        v[3], v[2], v[1], v[0], v[5], v[4], v[7], v[6], v[8], v[9], v[10], v[11], v[12], v[13], v[14], v[15]);
                },
                None => {
                    val = format!(" ");
                }
            }
        },
        JET_coltypDateTime => {
            assert!(c.cbmax as usize == size_of::<f64>());
            match get_column::<f64>(jdb, table_id, c.id)? {
                Some(v) => {
                    return Err(SimpleError::new(format!("VariantTimeToSystemTime failed")));
                    /*
                    let mut st = std::mem::MaybeUninit::<SYSTEMTIME>::zeroed();
                    unsafe {
                        let r = VariantTimeToSystemTime(v, st.as_mut_ptr());
                        if r == 1 {
                            let s = st.assume_init();
                            val = format!("{}.{}.{} {}:{}:{}", s.wDay, s.wMonth, s.wYear, s.wHour, s.wMinute, s.wSecond);
                        } else {
                            return Err(SimpleError::new(format!("VariantTimeToSystemTime failed")));
                        }
                    }
                    */
                },
                None => val = format!(" ")
            }
        },
        _ => {
            return Err(SimpleError::new(format!("Incorrect column type: {}, max is 19", c.typ)));
        }
    }
    Ok(val)
}

fn print_table(cols: &Vec<ColumnInfo>, rows: &Vec<Vec<String>>) {
    let mut col_sp : Vec<usize> = Vec::new();
    for i in 0..cols.len() {
        let mut col_max_sz = cols[i].name.len();
        for j in 0..rows.len() {
            if rows[j][i].len() > col_max_sz {
                col_max_sz = rows[j][i].len();
            }
        }
        col_sp.push(col_max_sz);
    }

    let mut nrow = String::new();
    for i in 0..cols.len() {
        nrow = format!("{}|{:2$}", nrow, cols[i].name, col_sp[i]);
    }
    println!("{}|", nrow);

    for i in 0..rows.len() {
        let mut row = String::new();
        for j in 0..rows[i].len() {
            row = format!("{}|{:2$}", row, rows[i][j], col_sp[j]);
        }
        println!("{}|", row);
    }
}

fn dump_table(jdb: &Box<dyn EseDb>, t: &str)
    -> Result<Option<(/*cols:*/Vec<ColumnInfo>, /*rows:*/Vec<Vec<String>>)>, SimpleError> {
    let table_id = jdb.open_table(&t)?;
    let cols = jdb.get_columns(&t)?;
    if !jdb.move_row(table_id, JET_MoveFirst as u32) {
        // empty table
        return Ok(None);
    }
    let mut rows : Vec<Vec<String>> = Vec::new();
    loop {
        let mut values : Vec<String> = Vec::new();
        for c in &cols {
            let val = get_column_val(jdb, table_id, &c);
            match val {
                Err(e) => {
                    println!("Error: {}", e);
                    values.push("".to_string());
                },
                Ok(v) => {
                    values.push(v);
                }
            }
        }
        assert_eq!(values.len(), cols.len());
        rows.push(values);
        if !jdb.move_row(table_id, JET_MoveNext) {
            break;
        }
    }
    jdb.close_table(table_id);
    Ok(Some((cols, rows)))
}

#[derive(PartialEq, Debug)]
pub enum Mode {
    EseApi,
    EseParser,
    Both
}

#[cfg(target_os = "windows")]
fn alloc_jdb(m: &Mode) -> Box<dyn EseDb> {
    if *m == Mode::EseApi {
        return Box::new(EseAPI::init());
    } else if *m == Mode::EseParser {
        return Box::new(EseParser::init(CACHE_SIZE_ENTRIES));
    }
    return Box::new(EseBoth::init());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn alloc_jdb(m: &Mode) -> Box<dyn EseDb> {
    if *m != Mode::EseParser {
        eprintln!("Unsupported mode: {:?}. EseAPI is available only under Windows build.", m);
        std::process::exit(-1);
    }
    return Box::new(EseParser::init(CACHE_SIZE_ENTRIES));
}

fn main() {
    let mut mode : Mode = {
        #[cfg(target_os = "windows")]
        { Mode::Both }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        { Mode::EseParser }
    };
    let mut table = String::new();
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 0 {
        eprintln!("db path required");
        return;
    }
    if args[0].find("help").is_some() {
        eprintln!("[/m mode] [/t table] db path");
        eprintln!("where mode one of [EseAPI, EseParser, *Both - default]");
        std::process::exit(0);
    }
    if args[0].to_lowercase() == "/m" {
        if args[1].to_lowercase() == "eseapi" {
            mode = Mode::EseApi;
        } else if args[1].to_lowercase() == "eseparser" {
            mode = Mode::EseParser;
        } else if args[1].to_lowercase() == "both" {
            mode = Mode::Both;
        } else {
            eprintln!("unknown mode: {}", args[1]);
            std::process::exit(-1);
        }
        args.drain(..2);
    }
    if args[0].to_lowercase() == "/t" {
        table = args[1].clone();
        args.drain(..2);
    }
    if args.len() == 0 {
        eprintln!("db path required");
        std::process::exit(-1);
    }
    let dbpath = args.concat();

    let mut jdb = alloc_jdb(&mode);
    println!("mode {:?}, path: {}", &mode, dbpath);

    let v = jdb.load(&dbpath);
    if v.is_some() {
        println!("Error: {:?}", v.unwrap());
        return ;
    }
    println!("loaded {}", dbpath);

    let handle_table = |t: &str| {
        println!("table {}", &t);
        match dump_table(&jdb, &t) {
            Ok(opt) => match opt {
                Some((cols, rows)) => print_table(&cols, &rows),
                None => println!("table {} is empty.", &t)
            },
            Err(e) => println!("table {}: {}", &t, e)
        }
    };

    if table.is_empty() {
        let tables = jdb.get_tables().unwrap();
        for t in tables {
            handle_table(&t);
        }
    } else {
        handle_table(&table);
    }
}
