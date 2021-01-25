use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::exceptions;
use pyo3::types::*;

use ese_parser_lib::{esent::*, ese_trait::*, ese_api::*};

use simple_error::SimpleError;

use std::ffi::OsString;
use std::os::windows::prelude::*;

use std::mem::MaybeUninit;

extern "C" {
    pub fn StringFromGUID2(
        rguid: *const ::std::os::raw::c_void,
        lpsz: *const ::std::os::raw::c_ushort,
        cchMax: ::std::os::raw::c_int
    ) -> ::std::os::raw::c_int;
}

#[pyclass]
pub struct PyEseDb {
    jdb : EseAPI,
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

#[pymethods]
impl PyEseDb {
    #[new]
    fn new() -> Self {
        PyEseDb {
            jdb : EseDb::init()
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

    fn move_row(&self, table: u64, crow: u32) -> bool {
        self.jdb.move_row(table, crow)
    }

    fn get_row(&self, table: u64, column: &PyColumnInfo) -> PyResult<Option<PyObject>> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        fn get<T>(s : &PyEseDb, table: u64, column: &PyColumnInfo) -> PyResult<Option<T>> {
            match s.jdb.get_column::<T>(table, column.id) {
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
            JET_coltypBit => {
                let n = get::<i8>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypUnsignedByte => {
                let n = get::<u8>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypShort => {
                let n = get::<i16>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypUnsignedShort => {
                let n = get::<u16>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypLong => {
                let n = get::<i32>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypUnsignedLong => {
                let n = get::<u32>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypLongLong => {
                let n = get::<i64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypUnsignedLongLong => {
                let n = get::<u64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypCurrency => { // TODO
                let n = get::<i64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypIEEESingle => {
                let n = get::<f32>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypIEEEDouble => {
                let n = get::<f64>(self, table, column)?;
                Ok(Some(n.to_object(py)))
            },
            JET_coltypBinary => {
                match self.jdb.get_column_dyn(table, column.id, column.cbmax as usize) {
                    Ok(ov) => {
                        match ov {
                            Some(n) => return Ok(Some(n.to_object(py))),
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            JET_coltypText => {
                match self.jdb.get_column_dyn(table, column.id, column.cbmax as usize) {
                    Ok(ov) => {
                        match ov {
                            Some(v) => {
                                if column.cp == 1200 {
                                    let t = v.as_slice();
                                    unsafe {
                                        let (_, v16, _) = t.align_to::<u16>();
                                        let ws = OsString::from_wide(&v16);
                                        let wss = ws.into_string().unwrap();
                                        return Ok(Some(wss.to_object(py)));
                                    }
                                } else {
                                    let s = std::str::from_utf8(&v).unwrap();
                                    return Ok(Some(s.to_object(py)));
                                }
                            }
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            JET_coltypLongText => {
                let d;
                if column.cbmax == 0 {
                    d = self.jdb.get_column_dyn_varlen(table, column.id);
                } else {
                    d = self.jdb.get_column_dyn(table, column.id, column.cbmax as usize);
                }
                match d {
                    Ok(ov) => {
                        match ov {
                            Some(v) => {
                                if column.cp == 1200 {
                                    let t = v.as_slice();
                                    unsafe {
                                        let (_, v16, _) = t.align_to::<u16>();
                                        let ws = OsString::from_wide(&v16);
                                        let wss = ws.into_string().unwrap();
                                        return Ok(Some(wss.to_object(py)));
                                    }
                                } else {
                                    let s = std::str::from_utf8(&v).unwrap();
                                    return Ok(Some(s.to_object(py)));
                                }
                            }
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            JET_coltypLongBinary => {
                let d;
                if column.cbmax == 0 {
                    d = self.jdb.get_column_dyn_varlen(table, column.id);
                } else {
                    d = self.jdb.get_column_dyn(table, column.id, column.cbmax as usize);
                }
                match d {
                    Ok(ov) => {
                        match ov {
                            Some(n) => return Ok(Some(n.to_object(py))),
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            JET_coltypGUID => {
                match self.jdb.get_column_dyn(table, column.id, column.cbmax as usize) {
                    Ok(ov) => {
                        match ov {
                            Some(v) => {
                                unsafe {
                                    let mut vstr : Vec<u16> = Vec::new();
                                    vstr.resize(39, 0);
                                    let r = StringFromGUID2(v.as_ptr() as *const std::os::raw::c_void,
                                        vstr.as_mut_ptr() as *const u16, vstr.len() as i32);
                                    if r > 0 {
                                        let mut s = OsString::from_wide(&vstr).into_string().unwrap();
                                        // remove new line
                                        s.pop();
                                        return Ok(Some(s.to_object(py)));
                                    }
                                }
                                return Err(PyErr::new::<exceptions::PyTypeError, _>("StringFromGUID2 failed"));
                            },
                            None => return Ok(None)
                        }
                    },
                    Err(e) => return Err(PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
                }
            },
            JET_coltypDateTime => {
                let ov = get::<f64>(self, table, column)?;
                match ov {
                    Some(v) => {
                        use winapi::um::minwinbase::SYSTEMTIME;
                        use winapi::shared::minwindef::FILETIME;
                        use winapi::shared::ntdef::LARGE_INTEGER;
                        use winapi::um::oleauto::VariantTimeToSystemTime;
                        use winapi::um::timezoneapi::SystemTimeToFileTime;
                        use std::io::Error;
                        unsafe {
                        let mut st = std::mem::zeroed::<SYSTEMTIME>();
                            let r = VariantTimeToSystemTime(v as winapi::shared::wtypesbase::DOUBLE, &mut st);
                            if r == 1 {
                                let mut ft = std::mem::zeroed::<FILETIME>();
                                let r = SystemTimeToFileTime(&mut st, &mut ft);
                                if r > 0 {
                                    let mut li = std::mem::zeroed::<LARGE_INTEGER>();
                                    let mut qli = li.s_mut();
                                    qli.LowPart = ft.dwLowDateTime;
                                    qli.HighPart = ft.dwHighDateTime as i32;
                                    // January 1, 1970 (start of Unix epoch) in "ticks"
                                    const UNIX_TIME_START : i64 = 0x019DB1DED53E8000;
                                    // a tick is 100ns
                                    const TICKS_PER_SECOND : i64 = 10000000;
                                    let unix_timestamp = (li.QuadPart() - UNIX_TIME_START) / TICKS_PER_SECOND;
                                    return Ok(Some(unix_timestamp.to_object(py)));
                                }
                            }
                            return Err(PyErr::new::<exceptions::PyTypeError, _>(format!("failed with error: {}", Error::last_os_error())));
                        }
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
    m.add_class::<PyEseDb>()?;
    Ok(())
}
