#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

mod utils;

use pyo3::exceptions;
use pyo3::prelude::*;

use log::LevelFilter;
use pyo3_log::Logger;

use crate::utils::*;
use ese_parser_lib::{
    ese_parser::FromBytes,
    ese_parser::ReadSeek,
    ese_parser::*,
    ese_trait::*};
use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;

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
    pub cp: u16,
}

#[pyclass]
pub struct PyEseDb {
    jdb: EseParser<Box<dyn ReadSeek + Send>>,
}

#[pymethods]
impl PyEseDb {
    #[new]
    fn new(path_or_file_like: PyObject) -> PyResult<Self> {
        let file_or_file_like = FileOrFileLike::from_pyobject(path_or_file_like)?;

        let boxed_read_seek = match file_or_file_like {
            FileOrFileLike::File(s) => {
                let file = File::open(s)?;
                let reader = BufReader::with_capacity(4096, file);
                Box::new(reader) as Box<dyn ReadSeek + Send>
            }
            FileOrFileLike::FileLike(f) => Box::new(f) as Box<dyn ReadSeek + Send>,
        };

        let parser = EseParser::load(10, boxed_read_seek)
            .map_err(|e| PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))?;

        Ok(Self { jdb: parser })
    }

    fn open_table(&self, table: &str) -> PyResult<u64> {
        self.jdb
            .open_table(table)
            .map_err(|e| PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
    }

    fn close_table(&self, table: u64) -> bool {
        self.jdb.close_table(table)
    }

    fn get_tables(&self) -> PyResult<Vec<String>> {
        self.jdb
            .get_tables()
            .map_err(|e| PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
    }

    fn get_columns(&self, table: &str) -> PyResult<Vec<PyColumnInfo>> {
        match self.jdb.get_columns(table) {
            Ok(t) => {
                let mut r: Vec<PyColumnInfo> = Vec::new();
                for i in &t {
                    let pc = PyColumnInfo {
                        name: i.name.to_string(),
                        id: i.id,
                        typ: i.typ,
                        cbmax: i.cbmax,
                        cp: i.cp,
                    };
                    r.push(pc);
                }
                Ok(r)
            }
            Err(e) => Err(PyErr::new::<exceptions::PyTypeError, _>(
                e.as_str().to_string(),
            )),
        }
    }

    fn get_column(&self, table: &str, column_name: &str) -> PyResult<PyColumnInfo> {
        match self.jdb.get_columns(table) {
            Ok(t) => {
                for i in &t {
                    if i.name == column_name {
                        return Ok(PyColumnInfo {
                            name: i.name.to_string(),
                            id: i.id,
                            typ: i.typ,
                            cbmax: i.cbmax,
                            cp: i.cp,
                        });
                    }
                }
                Err(PyErr::new::<exceptions::PyTypeError, _>(format!(
                    "no such column: {}",
                    column_name
                )))
            }
            Err(e) => Err(PyErr::new::<exceptions::PyTypeError, _>(
                e.as_str().to_string(),
            )),
        }
    }

    fn move_row(&self, table: u64, crow: i32) -> PyResult<bool> {
        self.jdb
            .move_row(table, crow)
            .map_err(|e| PyErr::new::<exceptions::PyTypeError, _>(e.as_str().to_string()))
    }

    fn get_value_mv(
        &self,
        table: u64,
        column: &PyColumnInfo,
        multi_value_index: u32,
    ) -> PyResult<Option<PyObject>> {
        Python::with_gil(|py| {
            let d = self.jdb.get_column_mv(table, column.id, multi_value_index);
            match d {
                Ok(ov) => match ov {
                    Some(n) => Ok(Some(n.to_object(py))),
                    None => Ok(None),
                },
                Err(e) => {
                    Err(PyErr::new::<exceptions::PyTypeError, _>(
                        e.as_str().to_string(),
                    ))
                }
            }
        })
    }

    fn get_value(&self, table: u64, column: &PyColumnInfo) -> PyResult<Option<PyObject>> {
        Python::with_gil(|py| {
            fn get<T: FromBytes>(
                s: &PyEseDb,
                table: u64,
                column: &PyColumnInfo,
            ) -> PyResult<Option<T>> {
                match s.jdb.get_fixed_column::<T>(table, column.id) {
                    Ok(ov) => match ov {
                        Some(n) => Ok(Some(n)),
                        None => Ok(None),
                    },
                    Err(e) => {
                        Err(PyErr::new::<exceptions::PyTypeError, _>(
                            e.as_str().to_string(),
                        ))
                    }
                }
            }

            match column.typ {
                ESE_coltypBit => {
                    let n = get::<i8>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypUnsignedByte => {
                    let n = get::<u8>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypShort => {
                    let n = get::<i16>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypUnsignedShort => {
                    let n = get::<u16>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypLong => {
                    let n = get::<i32>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypUnsignedLong => {
                    let n = get::<u32>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypLongLong => {
                    let n = get::<i64>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypUnsignedLongLong => {
                    let n = get::<u64>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypCurrency => {
                    // TODO
                    let n = get::<i64>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypIEEESingle => {
                    let n = get::<f32>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypIEEEDouble => {
                    let n = get::<f64>(self, table, column)?;
                    Ok(Some(n.to_object(py)))
                }
                ESE_coltypBinary => match self.jdb.get_column(table, column.id) {
                    Ok(ov) => match ov {
                        Some(n) => Ok(Some(n.to_object(py))),
                        None => Ok(None),
                    },
                    Err(e) => {
                        Err(PyErr::new::<exceptions::PyTypeError, _>(
                            e.as_str().to_string(),
                        ))
                    }
                },
                ESE_coltypText => match self.jdb.get_column(table, column.id) {
                    Ok(ov) => match ov {
                        Some(v) => {
                            let unicode = ESE_CP::try_from(column.cp) == Ok(ESE_CP::Unicode);
                            Ok(utils::bytes_to_string(v, unicode).map(|s| s.to_object(py)))
                        }
                        None => Ok(None),
                    },
                    Err(e) => {
                        Err(PyErr::new::<exceptions::PyTypeError, _>(
                            e.as_str().to_string(),
                        ))
                    }
                },
                ESE_coltypLongText => match self.jdb.get_column(table, column.id) {
                    Ok(ov) => match ov {
                        Some(v) => {
                            let unicode = ESE_CP::try_from(column.cp) == Ok(ESE_CP::Unicode);
                            Ok(utils::bytes_to_string(v, unicode).map(|s| s.to_object(py)))
                        }
                        None => Ok(None),
                    },
                    Err(e) => {
                        Err(PyErr::new::<exceptions::PyTypeError, _>(
                            e.as_str().to_string(),
                        ))
                    }
                },
                ESE_coltypLongBinary => match self.jdb.get_column(table, column.id) {
                    Ok(ov) => match ov {
                        Some(n) => Ok(Some(n.to_object(py))),
                        None => Ok(None),
                    },
                    Err(e) => {
                        Err(PyErr::new::<exceptions::PyTypeError, _>(
                            e.as_str().to_string(),
                        ))
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
                                    Ok(Some(val.to_object(py)))
                                }
                                None => Ok(None),
                            }
                        }
                        Err(e) => {
                            Err(PyErr::new::<exceptions::PyTypeError, _>(
                                e.as_str().to_string(),
                            ))
                        }
                    }
                }
                ESE_coltypDateTime => match self.jdb.get_column_date(table, column.id) {
                    Ok(ov) => match ov {
                        Some(v) => Ok(Some(date_to_pyobject(&v)?)),
                        None => Ok(None),
                    },
                    Err(e) => {
                        Err(PyErr::new::<exceptions::PyTypeError, _>(
                            e.as_str().to_string(),
                        ))
                    }
                },
                _ => {
                    Err(PyErr::new::<exceptions::PyTypeError, _>(format!(
                        "Unknown type {}, column: {}, id: {}, cbmax: {}, cp: {}",
                        column.typ, column.name, column.id, column.cbmax, column.cp
                    )))
                }
            }
        })
    }

    // ASDF-5542: new name for same API
    fn get_row(&self, table: u64, column: &PyColumnInfo) -> PyResult<Option<PyObject>> {
        log::warn!("`get_row` is deprecated; please use `get_value`");
        self.get_value(table, column)
    }

    // ASDF-5542: new name for same API
    fn get_row_mv(
        &self,
        table: u64,
        column: &PyColumnInfo,
        multi_value_index: u32,
    ) -> PyResult<Option<PyObject>> {
        log::warn!("`get_row_mv` is deprecated; please use `get_value_mv`");
        self.get_value_mv(table, column, multi_value_index)
    }
}

#[pymodule]
fn ese_parser(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    let pylogger = Logger::default().filter(LevelFilter::Warn);
    if let Err(why) = pylogger.install() {
        eprintln!("Error initializing Python logger for PyEseDb: {}", why)
    }

    m.add_class::<PyEseDb>()?;
    Ok(())
}
