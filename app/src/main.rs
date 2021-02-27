#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types, clippy::mut_from_ref, clippy::cast_ptr_alignment)]

use std::env;

use ese_parser_lib::{esent::*, ese_trait::*, ese_api::*, ese_parser::*};

use std::mem::{size_of, MaybeUninit};
use std::cell::RefCell;

use std::ffi::OsString;
use std::os::windows::prelude::*;

use simple_error::SimpleError;

const CACHE_SIZE_ENTRIES : usize = 10;

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

extern "C" {
    pub fn RtlCompareMemory(
        pvArg1: *mut ::std::os::raw::c_void,
        pvArg2: *mut ::std::os::raw::c_void,
        Length: usize
    ) -> usize;
}

struct EseBoth {
    api: EseAPI,
    parser : EseParser,
    opened_tables: RefCell<Vec<(u64, u64)>>,
}

impl EseBoth {
    pub fn init() -> EseBoth {
        EseBoth { api: EseAPI::init(), parser: EseParser::init(CACHE_SIZE_ENTRIES), opened_tables: RefCell::new(Vec::new()) }
    }
}

impl EseDb for EseBoth {
    fn load(&mut self, dbpath: &str) -> Option<SimpleError> {
        if let Some(e) = self.api.load(dbpath) {
            return Some(SimpleError::new(format!("EseAPI::load failed: {}", e)));
        }
        if let Some(e) = self.parser.load(dbpath) {
            return Some(SimpleError::new(format!("EseParser::load failed: {}", e)));
        }
        None
    }

    fn error_to_string(&self, err: i32) -> String {
        "useless".to_string()
    }

    fn open_table(&self, table: &str) -> Result<u64, SimpleError> {
        let api_table = self.api.open_table(table).map_err(|e| SimpleError::new(format!("EseAPI::open_table failed: {}", e)))?;
        let parser_table = self.parser.open_table(table).map_err(|e| SimpleError::new(format!("EseParser::open_table failed: {}", e)))?;
        let mut v = self.opened_tables.borrow_mut();
        v.push((api_table, parser_table));
        Ok((v.len()-1) as u64)
    }

    fn close_table(&self, table: u64) -> bool {
        let mut t = self.opened_tables.borrow_mut();
        let (api_table, parser_table) = t[table as usize];
        if !self.api.close_table(api_table) {
            println!("EseAPI::close_table({}) failed", api_table);
            return false;
        }
        if !self.parser.close_table(parser_table) {
            println!("EseParser::close_table({}) failed", parser_table);
            return false;
        }
        t.remove(table as usize);
        true
    }

    fn get_tables(&self) -> Result<Vec<String>, SimpleError> {
        let api_tables = self.api.get_tables().map_err(|e| SimpleError::new(format!("EseAPI::get_tables failed: {}", e)))?;
        let parser_tables = self.parser.get_tables().map_err(|e| SimpleError::new(format!("EseParser::get_tables failed: {}", e)))?;
        if api_tables.len() != parser_tables.len() {
            return Err(SimpleError::new(format!("get_tables() have difference: EseAPI tables:\n{:?}\n not equal to EseParser:\n{:?}\n",
                api_tables, parser_tables)));
        }
        for i in 0..api_tables.len() {
            if api_tables[i] != parser_tables[i] {
                return Err(SimpleError::new(format!("get_tables() have difference: EseAPI table:\n{:?}\n not equal to EseParser:\n{:?}\n",
                    api_tables[i], parser_tables[i])));
            }
        }
        Ok(api_tables)
    }

    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError> {
        let api_columns = self.api.get_columns(table).map_err(|e| SimpleError::new(format!("EseAPI::get_columns failed: {}", e)))?;
        let parser_columns = self.parser.get_columns(table).map_err(|e| SimpleError::new(format!("EseParser::get_columns failed: {}", e)))?;
        if api_columns.len() != parser_columns.len() {
            return Err(SimpleError::new(format!("get_columns({}) have difference: EseAPI columns:\n{:?}\n not equal to EseParser:\n{:?}\n",
                table, api_columns, parser_columns)));
        }
        for i in 0..api_columns.len() {
            if api_columns[i].id == parser_columns[i].id {
                let c1 = &api_columns[i];
                let c2 = &parser_columns[i];
                if c1.name  != c2.name  ||
                   c1.typ   != c2.typ   ||
                   c1.cbmax != c2.cbmax ||
                   c1.cp    != c2.cp
                {
                    return Err(SimpleError::new(format!("get_columns({}) have difference: EseAPI table:\n{:?}\n not equal to EseParser:\n{:?}\n",
                        table, api_columns[i], parser_columns[i])));
                }
            }
        }
        // sorted by id
        Ok(parser_columns)
    }

    fn move_row(&self, table: u64, crow: u32) -> bool {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let r1 = self.api.move_row(api_table, crow);
        let r2 = self.parser.move_row(parser_table, crow);
        if r1 != r2 {
            println!("move_row return result different: EseAPI {} != EseParser {}", r1, r2);
        }
        r1
    }

    fn get_column_str(&self, table: u64, column: u32, size: u32) -> Result<Option<String>, SimpleError> {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let s1 = self.api.get_column_str(api_table, column, size)?;
        let s2 = self.parser.get_column_str(parser_table, column, size)?;
        if s1 != s2 {
            return Err(SimpleError::new(format!(r"table {}, column({}) EseAPI column '{:?}' not equal to EseParser '{:?}'",
                table, column, s1, s2)));
        }
        Ok(s1)
    }

    fn get_column_dyn(&self, table: u64, column: u32, size: usize) -> Result< Option<Vec<u8>>, SimpleError> {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let s1 = self.api.get_column_dyn(api_table, column, size)?;
        let s2 = self.parser.get_column_dyn(parser_table, column, size)?;
        if s1 != s2 {
            return Err(SimpleError::new(format!(r"table {}, column({}) EseAPI column '{:?}' not equal to EseParser '{:?}'",
                table, column, s1, s2)));
        }
        Ok(s1)
    }

    fn get_column_dyn_varlen(&self, table: u64, column: u32) -> Result< Option<Vec<u8>>, SimpleError> {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let s1 = self.api.get_column_dyn_varlen(api_table, column)?;
        let s2 = self.parser.get_column_dyn_varlen(parser_table, column)?;
        if s1 != s2 {
            return Err(SimpleError::new(format!(r"table {}, column({}) EseAPI column '{:?}' not equal to EseParser '{:?}'",
                table, column, s1, s2)));
        }
        Ok(s1)
    }
}

fn get_column<T>(jdb: &Box<dyn EseDb>, table: u64, column: u32) -> Result<Option<T>, SimpleError> {
    let size = std::mem::size_of::<T>();
    let mut dst = std::mem::MaybeUninit::<T>::zeroed();

    let vo = jdb.get_column_dyn(table, column, size)?;

    unsafe {
        if let Some(v) = vo {
            std::ptr::copy_nonoverlapping(
                v.as_ptr(),
                dst.as_mut_ptr() as *mut u8,
                size);
        }
        return Ok(Some(dst.assume_init()));
    }
}

fn dump_table(jdb: &Box<dyn EseDb>, t: &str) {
    let table_id = jdb.open_table(&t).unwrap();
    let cols = jdb.get_columns(&t).unwrap();
    if !jdb.move_row(table_id, JET_MoveFirst as u32) {
        // empty table
        return ;
    }
    let mut rows : Vec<Vec<String>> = Vec::new();
    loop {
        let mut values : Vec<String> = Vec::new();
        for c in &cols {
            let mut val = String::new();
            match c.typ {
                JET_coltypBit => {
                    assert!(c.cbmax as usize == size_of::<i8>());
                    match get_column::<i8>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypUnsignedByte => {
                    assert!(c.cbmax as usize == size_of::<u8>());
                    match get_column::<u8>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypShort => {
                    assert!(c.cbmax as usize == size_of::<i16>());
                    match get_column::<i16>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypUnsignedShort => {
                    assert!(c.cbmax as usize == size_of::<u16>());
                    match get_column::<u16>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypLong => {
                    assert!(c.cbmax as usize == size_of::<i32>());
                    match get_column::<i32>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypUnsignedLong => {
                    assert!(c.cbmax as usize == size_of::<u32>());
                    match get_column::<u32>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypLongLong => {
                    assert!(c.cbmax as usize == size_of::<i64>());
                    match get_column::<i64>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypUnsignedLongLong => {
                    assert!(c.cbmax as usize == size_of::<u64>());
                    match get_column::<u64>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypCurrency => {
                    assert!(c.cbmax as usize == size_of::<i64>());
                    match get_column::<i64>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypIEEESingle => {
                    assert!(c.cbmax as usize == size_of::<f32>());
                    match get_column::<f32>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypIEEEDouble => {
                    assert!(c.cbmax as usize == size_of::<f64>());
                    match get_column::<f64>(jdb, table_id, c.id).unwrap() {
                        Some(v) => val = format!("{}", v),
                        None => val = format!(" ")
                    }
                },
                JET_coltypBinary => {
                    match jdb.get_column_dyn(table_id, c.id, c.cbmax as usize) {
                        Ok(ov) => {
                            match ov {
                                Some(v) => {
                                    let s = v.iter().map(|c| format!("{:x?} ", c).to_string() ).collect::<String>();
                                    val = format!("{} ", s);
                                },
                                None => {
                                    val = format!(" ");
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
                                            val = format!("{}", wss);
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
                                            if wss.len() > 20 {
                                                val = format!("{:4} bytes: {}...", wss.len(), truncate(&wss, 32).to_string());
                                            } else {
                                                val = format!("{}", wss);
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
                                            val = format!("{} ", s);
                                        }
                                    }
                                },
                                None => {
                                    val = format!(" ");
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
                    match get_column::<f64>(jdb, table_id, c.id).unwrap() {
                        Some(v) => {
                            let mut st = MaybeUninit::<SYSTEMTIME>::zeroed();
                            unsafe {
                                let r = VariantTimeToSystemTime(v, st.as_mut_ptr());
                                if r == 1 {
                                    let s = st.assume_init();
                                    val = format!("{}.{}.{} {}:{}:{}", s.wDay, s.wMonth, s.wYear, s.wHour, s.wMinute, s.wSecond);
                                } else {
                                    println!("VariantTimeToSystemTime failed");
                                }
                            }
                        },
                        None => val = format!(" ")
                    }
                },
                _ => {
                    println!("Incorrect column type: {}, max is 19", c.typ);
                }
            }
            //println!("{}", val);
            values.push(val);
        }
        assert_eq!(values.len(), cols.len());
        rows.push(values);
        if !jdb.move_row(table_id, JET_MoveNext) {
            break;
        }
    }
    jdb.close_table(table_id);

    // print table

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

#[derive(PartialEq, Debug)]
pub enum Mode {
    EseApi,
    EseParser,
    Both
}
fn alloc_jdb(m: &Mode) -> Box<dyn EseDb> {
    if *m == Mode::EseApi {
        return Box::new(EseAPI::init());
    } else if *m == Mode::EseParser {
        return Box::new(EseParser::init(CACHE_SIZE_ENTRIES));
    }
    return Box::new(EseBoth::init());
}

fn main() {
    let mut mode : Mode = Mode::Both;
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

    if table.is_empty() {
        let tables = jdb.get_tables().unwrap();
        for t in tables {
            println!("table {}", &t);
            dump_table(&jdb, &t);
        }
    } else {
        println!("table {}", &table);
        dump_table(&jdb, &table);
    }
}