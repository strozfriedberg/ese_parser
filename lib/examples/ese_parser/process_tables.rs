extern crate ese_parser_lib;

#[cfg(all(feature = "nt_comparison", target_os = "windows"))]
use ese_parser_lib::parser::ese_both::*;
use ese_parser_lib::{ese_parser::*, ese_trait::*, vartime::*};
use simple_error::SimpleError;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{self, Error, Write};
use std::mem::size_of;
use std::path::PathBuf;
use std::result::Result;

fn get_column<T: FromBytes>(
    jdb: &dyn EseDb,
    table: u64,
    column: u32,
) -> Result<Option<T>, SimpleError> {
    match jdb.get_column(table, column)? {
        Some(v) => Ok(Some(T::from_bytes(&v))),
        None => Ok(None),
    }
}

fn get_column_val(jdb: &dyn EseDb, table_id: u64, c: &ColumnInfo) -> Result<String, SimpleError> {
    use std::fmt::Write;
    let val;
    match c.typ {
        ESE_coltypBit => {
            assert!(c.cbmax as usize == size_of::<i8>());
            match get_column::<i8>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypUnsignedByte => {
            assert!(c.cbmax as usize == size_of::<u8>());
            match get_column::<u8>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypShort => {
            assert!(c.cbmax as usize == size_of::<i16>());
            match get_column::<i16>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypUnsignedShort => {
            assert!(c.cbmax as usize == size_of::<u16>());
            match get_column::<u16>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypLong => {
            assert!(c.cbmax as usize == size_of::<i32>());
            match get_column::<i32>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypUnsignedLong => {
            assert!(c.cbmax as usize == size_of::<u32>());
            match get_column::<u32>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypLongLong => {
            assert!(c.cbmax as usize == size_of::<i64>());
            match get_column::<i64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypUnsignedLongLong => {
            assert!(c.cbmax as usize == size_of::<u64>());
            match get_column::<u64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypCurrency => {
            assert!(c.cbmax as usize == size_of::<i64>());
            match get_column::<i64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypIEEESingle => {
            assert!(c.cbmax as usize == size_of::<f32>());
            match get_column::<f32>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypIEEEDouble => {
            assert!(c.cbmax as usize == size_of::<f64>());
            match get_column::<f64>(jdb, table_id, c.id)? {
                Some(v) => val = format!("{}", v),
                None => val = (" ").to_string(),
            }
        }
        ESE_coltypBinary => match jdb.get_column(table_id, c.id)? {
            Some(v) => {
                let s = v.iter().fold(String::new(), |mut out, c| {
                    let _ = write!(out, "{:x?} ", c);
                    out
                });
                val = format!("{} ", s);
            }
            None => {
                val = (" ").to_string();
            }
        },
        ESE_coltypText | ESE_coltypLongText => match jdb.get_column(table_id, c.id)? {
            Some(v) => {
                if ESE_CP::try_from(c.cp) == Ok(ESE_CP::Unicode) {
                    match ese_parser_lib::utils::from_utf16(&v) {
                        Ok(s) => val = s,
                        Err(e) => val = format!("from_utf16 failed: {}", e),
                    }
                } else {
                    match std::str::from_utf8(&v) {
                        Ok(s) => val = s.to_string(),
                        Err(e) => val = format!("from_utf8 failed: {}", e),
                    }
                }
            }
            None => {
                val = (" ").to_string();
            }
        },
        ESE_coltypLongBinary => match jdb.get_column(table_id, c.id)? {
            Some(mut v) => {
                let orig_size = v.len();
                v.truncate(16);
                let s = v.iter().fold(String::new(), |mut out, c| {
                    let _ = write!(out, "{:02x}  ", c);
                    out
                });
                val = format!("{:4} bytes: {}...", orig_size, s);
            }
            None => {
                val = (" ").to_string();
            }
        },
        ESE_coltypGUID => {
            match jdb.get_column(table_id, c.id)? {
                Some(v) => {
                    // {CD2C96BD-DCA8-47CB-B829-8F1AE4E2E686}
                    val = format!("{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
                        v[3], v[2], v[1], v[0], v[5], v[4], v[7], v[6], v[8], v[9], v[10], v[11], v[12], v[13], v[14], v[15]);
                }
                None => {
                    val = (" ").to_string();
                }
            }
        }
        ESE_coltypDateTime => {
            assert!(c.cbmax as usize == size_of::<f64>());
            match get_column::<u64>(jdb, table_id, c.id)? {
                Some(filetime) => {
                    let datetime = get_date_time_from_filetime(filetime);
                    val = format!("{}", datetime);
                }
                None => val = (" ").to_string(),
            }
        }
        _ => {
            return Err(SimpleError::new(format!(
                "Incorrect column type: {}, max is 19",
                c.typ
            )));
        }
    }
    Ok(val)
}

type Row = Vec<Vec<String>>;
type Col = Vec<ColumnInfo>;
type Table = (Col, Row);

fn dump_table(jdb: &dyn EseDb, t: &str) -> Result<Option<Table>, SimpleError> {
    let table_id = jdb.open_table(t)?;
    let cols = jdb.get_columns(t)?;
    if !jdb.move_row(table_id, ESE_MoveFirst)? {
        // empty table
        return Ok(None);
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    loop {
        let mut values: Vec<String> = Vec::new();
        for c in &cols {
            let val = get_column_val(jdb, table_id, c);
            match val {
                Err(e) => {
                    println!("Error: {}", e);
                    values.push("".to_string());
                }
                Ok(v) => {
                    values.push(v);
                }
            }
        }
        assert_eq!(values.len(), cols.len());
        rows.push(values);
        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    jdb.close_table(table_id);
    Ok(Some((cols, rows)))
}

const CACHE_SIZE_ENTRIES: usize = 10;

#[allow(unreachable_code)]
#[cfg(target_os = "windows")]
fn alloc_jdb(m: &Mode, dbpath: &str) -> Box<dyn EseDb> {
    #[cfg(feature = "nt_comparison")]
    {
        use ese_parser_lib::esent::ese_api::EseAPI;

        if *m == Mode::EseApi {
            return Box::new(EseAPI::load_from_path(dbpath).unwrap());
        } else if *m == Mode::EseParser {
            return Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, dbpath).unwrap());
        } else {
            return Box::new(EseBoth::load_from_path(dbpath).unwrap());
        }
    }
    // else
    alloc_jdb_ese_parser(m, dbpath)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn alloc_jdb(m: &Mode, dbpath: &str) -> Box<dyn EseDb> {
    alloc_jdb_ese_parser(m, dbpath)
}

fn alloc_jdb_ese_parser(m: &Mode, dbpath: &str) -> Box<dyn EseDb> {
    if *m != Mode::EseParser {
        eprintln!(
            "Unsupported mode: {:?}. EseAPI is available only under Windows \"nt_comparison\" feature build.",
            m
        );
        std::process::exit(-1);
    }
    Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, dbpath).unwrap())
}

fn print_table(
    cols: &[ColumnInfo],
    rows: &[Vec<String>],
    output_destination: &mut dyn std::io::Write,
) {
    let mut col_sp: Vec<usize> = Vec::new();
    for (i, col) in cols.iter().enumerate() {
        let mut col_max_sz = col.name.len();
        for row in rows.iter() {
            if row[i].len() > col_max_sz {
                col_max_sz = row[i].len();
            }
        }
        col_sp.push(col_max_sz);
    }

    let mut nrow = String::new();
    for (i, col) in cols.iter().enumerate() {
        nrow = format!("{}|{:2$}", nrow, col.name, col_sp[i]);
    }
    writeln!(output_destination, "{}|", nrow).unwrap();

    for r in rows.iter() {
        let mut row = String::new();
        for (j, r2) in r.iter().enumerate() {
            row = format!("{}|{:2$}", row, r2, col_sp[j]);
        }
        writeln!(output_destination, "{}|", row).unwrap();
    }
}

#[derive(PartialEq, Debug)]
pub enum Mode {
    EseApi,
    EseParser,
    Both,
}

pub fn resolve_path(test_file: Option<PathBuf>) -> Result<Box<dyn Write>, Error> {
    match test_file {
        Some(ref path) => File::create(path).map(|f| Box::new(f) as Box<dyn Write>),
        None => Ok(Box::new(io::stdout())),
    }
}

pub fn process_table(dbpath: &str, test_file: Option<PathBuf>, mode: Mode, table: String) {
    let mut output_destination = resolve_path(test_file).unwrap();
    println!("mode {:?}, path: {}", &mode, dbpath);
    let jdb = alloc_jdb(&mode, dbpath);
    println!("loaded {}", dbpath);
    //let output_destination = output_destination.clone();
    let mut handle_table = |t: &str| {
        writeln!(output_destination, "table {}", &t).unwrap();
        match dump_table(&*jdb, t) {
            Ok(opt) => match opt {
                Some((cols, rows)) => print_table(&cols, &rows, &mut output_destination),
                None => writeln!(output_destination, "table {} is empty.", &t).unwrap(),
            },
            Err(e) => writeln!(output_destination, "table {}: {}", &t, e).unwrap(),
        }
    };
    if table.is_empty() {
        let tables = jdb.get_tables().expect("Tables not found");
        for t in tables {
            handle_table(&t);
        }
    } else {
        handle_table(&table);
    }
}

use std::convert::TryInto;

pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl FromBytes for i8 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i8::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u8 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u8::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for i16 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i16::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u16 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u16::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for i32 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u32 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for i64 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i64::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u64 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u64::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for f32 {
    fn from_bytes(bytes: &[u8]) -> Self {
        f32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for f64 {
    fn from_bytes(bytes: &[u8]) -> Self {
        f64::from_le_bytes(bytes.try_into().unwrap())
    }
}
