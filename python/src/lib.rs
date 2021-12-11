#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use pyo3::prelude::*;
use pyo3::{exceptions, wrap_pyfunction};
use pyo3::callback::convert;

use ese_parser_lib::{ese_trait::*, ese_parser::*, ese_parser::FromBytes, vartime::*};
use widestring::U16String;
use std::convert::{TryFrom, TryInto};
use chrono::{DateTime, Utc};

#[pyclass]
pub struct PyEseDb {
    jdb : EseParser,
}

#[pyclass]
pub struct PyColumnInfo {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub id: u32,
    #[pyo3(get)]
    pub typ: u32,
    #[pyo3(get)]
    pub cbmax: u32,
    #[pyo3(get)]
    pub cp: u16
}

fn bytes_to_string(v: Vec<u8>, wide: bool) -> Option<String> {
    if wide {
        let t = v.as_slice();
        unsafe {
            let (_, v16, _) = t.align_to::<u16>();
            Some(U16String::from_ptr(v16.as_ptr(), v16.len()).to_string_lossy())
        }
    } else {
        std::str::from_utf8(&v).map(|s| s.to_string()).ok()
    }
}

fn SystemTimeToFileTime(st: &SYSTEMTIME) -> i64 {
    const TICKSPERMSEC : i64  = 10000;
    const TICKSPERSEC : i64 = 10000000;
    const SECSPERDAY : i64 = 86400;
    const SECSPERHOUR : i64 = 3600;
    const SECSPERMIN : i64 = 60;

    let monthLengths = vec![
        vec![31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
        vec![31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    ];

    fn IsLeapYear(year: i32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    fn DaysSinceEpoch(mut year: i32) -> i64 {
        const DAYSPERNORMALYEAR : i32 = 365;
        const EPOCHYEAR : i32 = 1601;
        year -= 1; // Don't include a leap day from the current year
        let mut days = year * DAYSPERNORMALYEAR + year / 4 - year / 100 + year / 400;
        days -= (EPOCHYEAR - 1) * DAYSPERNORMALYEAR + (EPOCHYEAR - 1) / 4 - (EPOCHYEAR - 1) / 100 + (EPOCHYEAR - 1) / 400;
        days as i64
    }

    let mut t : i64 = DaysSinceEpoch(st.wYear as i32);
    for curMonth in 1..st.wMonth {
        t += monthLengths[IsLeapYear(st.wYear as i32) as usize][curMonth as usize  - 1];
    }
    t += st.wDay as i64 - 1;
    t *= SECSPERDAY;
    t += st.wHour as i64 * SECSPERHOUR + st.wMinute as i64 * SECSPERMIN + st.wSecond as i64;
    t *= TICKSPERSEC;
    t += st.wMilliseconds as i64 * TICKSPERMSEC;
    t
}

#[pyfunction]
pub fn wrap_date_time_from_filetime(column_contents: Vec<u8>) -> String {
    let datetime = format!("{}", get_date_time_from_filetime(u64::from_le_bytes(column_contents.try_into().unwrap())));
    datetime
}

pub fn get_date_time_from_filetime(filetime: u64) -> DateTime<Utc> {
    const UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH: i128 = 11644473600;
    const UNIX_EPOCH_NANOS: i128 = UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH * 1_000_000_000;
    let filetime_nanos: i128 = filetime as i128 * 100;

    // Add nanoseconds to timestamp via Duration
    DateTime::<Utc>::from_utc(
        chrono::NaiveDate::from_ymd(1970, 1, 1).and_hms_nano(0, 0, 0, 0)
            + chrono::Duration::nanoseconds((filetime_nanos - UNIX_EPOCH_NANOS) as i64),
        Utc,
    )
}

#[pymethods]
impl PyEseDb {
    #[new]
    fn new() -> Self {
        PyEseDb {
            jdb : EseParser::init(10)
        }
    }

    fn load(&mut self, dbpath: &str) -> Option<String> {
        match self.jdb.load(dbpath) {
            Some(x) => Some(x.as_str().to_string()),
            None => None
        }
    }

    fn open_table(&self, table: &str) -> PyResult<u64> {
        match self.jdb.open_table(table) {
            Ok(v) => Ok(v),
            Err(e) => Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
        }
    }

    fn close_table(&self, table: u64) -> bool {
        self.jdb.close_table(table)
    }

    fn get_tables(&self) -> PyResult<Vec<String>> {
        match self.jdb.get_tables() {
            Ok(t) => Ok(t),
            Err(e) => Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
        }
    }

    fn get_columns(&self, table: &str) -> PyResult<Vec<PyColumnInfo>> {
        match self.jdb.get_columns(table) {
            Ok(t) => {
                let mut r : Vec<PyColumnInfo> = Vec::new();
                for i in &t {
                    let pc = PyColumnInfo { name: i.name.to_string(), id: i.id, typ: i.typ, cbmax: i.cbmax, cp: i.cp };
                    r.push(pc);
                }
                Ok(r)
            },
            Err(e) => Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
        }
    }

    fn get_column(&self, table: &str, column_name : &str) -> PyResult<PyColumnInfo> {
        match self.jdb.get_columns(table) {
            Ok(t) => {
                for i in &t {
                    if i.name == column_name {
                        return Ok(PyColumnInfo {
                            name: i.name.to_string(), id: i.id, typ: i.typ, cbmax: i.cbmax, cp: i.cp });
                    }
                }
                Err(PyErr::new::<exceptions::PyTypeError, _>(format!("no such column: {}", column_name)))
            },
            Err(e) => Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
        }
    }

    fn move_row(&self, table: u64, crow: i32) -> bool {
        self.jdb.move_row(table, crow)
    }

    fn get_row_mv(&self, table: u64, column: &PyColumnInfo, multi_value_index: u32) -> PyResult<Option<PyObject>> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let d = self.jdb.get_column_mv(table, column.id, multi_value_index);
        match d {
            Ok(ov) => {
                match ov {
                    Some(n) => return Ok(Some(n.to_object(py))),
                    None => return Ok(None)
                }
            },
            Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
        }
    }

    fn get_row(&self, table: u64, column: &PyColumnInfo) -> PyResult<Option<PyObject>> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        fn get<T: FromBytes>(s : &PyEseDb, table: u64, column: &PyColumnInfo) -> PyResult<Option<T>> {
            match s.jdb.get_fixed_column::<T>(table, column.id) {
                Ok(ov) => {
                    match ov {
                        Some(n) => return Ok(Some(n)),
                        None => return Ok(None)
                    }
                },
                Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
            }
        }

        match column.typ {
            ESE_coltypBit => {
                let n = get::<i8>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypUnsignedByte => {
                let n = get::<u8>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypShort => {
                let n = get::<i16>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypUnsignedShort => {
                let n = get::<u16>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypLong => {
                let n = get::<i32>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypUnsignedLong => {
                let n = get::<u32>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypLongLong => {
                let n = get::<i64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypUnsignedLongLong => {
                let n = get::<u64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypCurrency => { // TODO
                let n = get::<i64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypIEEESingle => {
                let n = get::<f32>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypIEEEDouble => {
                let n = get::<f64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            ESE_coltypBinary => {
                match self.jdb.get_column(table, column.id) {
                    Ok(ov) => {
                        match ov {
                            Some(n) => return Ok(Some(n.to_object(py))),
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            ESE_coltypText => {
                match self.jdb.get_column(table, column.id) {
                    Ok(ov) => {
                        match ov {
                            Some(v) => {
                                let unicode = ESE_CP::try_from(column.cp) == Ok(ESE_CP::Unicode);
                                return Ok(bytes_to_string(v, unicode).map(|s| s.to_object(py)));
                            }
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            ESE_coltypLongText => {
                match self.jdb.get_column(table, column.id) {
                    Ok(ov) => {
                        match ov {
                            Some(v) => {
                                let unicode = ESE_CP::try_from(column.cp) == Ok(ESE_CP::Unicode);
                                return Ok(bytes_to_string(v, unicode).map(|s| s.to_object(py)));
                            }
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            ESE_coltypLongBinary => {
                match self.jdb.get_column(table, column.id) {
                    Ok(ov) => {
                        match ov {
                            Some(n) => return Ok(Some(n.to_object(py))),
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            ESE_coltypGUID => {
                match self.jdb.get_column(table, column.id) {
                    Ok(ov) => {
                        match ov {
                            Some(v) => {
                                // {CD2C96BD-DCA8-47CB-B829-8F1AE4E2E686}
                                let val = format!("{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
                                    v[3], v[2], v[1], v[0], v[5], v[4], v[7], v[6], v[8], v[9], v[10], v[11], v[12], v[13], v[14], v[15]);
                                return Ok(Some(val.to_object(py)));
                            },
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            ESE_coltypDateTime => {
                let ov = get::<f64>(self, table, column)?;
                match ov {
                    Some(filetime) => {
                    let datetime = get_date_time_from_filetime(filetime);
                    val = format!(
                        "{}",
                        datetime
                        );
                        return Ok(datetime);
                    },
                    None => return Ok(None)
                }
            },
            _ => {
                return Err(PyErr::new::<exceptions::PyTypeError, _>(
                    format!("Unknown type {}, column: {}, id: {}, cbmax: {}, cp: {}",
                        column.typ, column.name, column.id, column.cbmax, column.cp)))
            }
        }
    }
}

#[pymodule]
fn ese_parser(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyEseDb>();
    m.add_function(wrap_pyfunction!(wrap_date_time_from_filetime, m)?)?;
    Ok(())
}
