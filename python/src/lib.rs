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

#[pyclass]
pub struct PyEseDb {
    jdb : EseAPI,
}

#[pyclass]
pub struct PyColumnInfo {
    pub name: String,
    pub id: u32,
    pub typ: u32,
    pub cbmax: u32,
    pub cp: u16
}

#[pymethods]
impl PyColumnInfo {
    fn name(&self) -> String { self.name.to_string() }
    fn id(&self) -> u32 { self.id }
    fn typ(&self) -> u32 { self.typ }
    fn cbmax(&self) -> u32 { self.cbmax }
    fn cp(&self) -> u16 { self.cp }
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

    fn get_row(&self, table: u64, column: &PyColumnInfo) -> PyResult<Option<Py<PyAny>>> {
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
                        let mut st = MaybeUninit::<SYSTEMTIME>::zeroed();
                        unsafe {
                            let r = VariantTimeToSystemTime(v, st.as_mut_ptr());
                            if r == 1 {
                                let s = st.assume_init();
                                let pydt = PyDateTime::new(py,
                                    s.wYear as i32, s.wMonth as u8, s.wDay as u8,
                                    s.wHour as u8, s.wMinute as u8, s.wSecond as u8,
                                    s.wMilliseconds as u32, None).unwrap();
                                return Ok(Some(pydt.to_object(py)));
                            } else {
                                return Err(PyErr::new::<exceptions::PyTypeError, _>("VariantTimeToSystemTime failed"));
                            }
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
