use pyo3::types::{PyDateTime, PyString};
use pyo3::ToPyObject;
use pyo3::{PyObject, PyResult, Python};
use pyo3_file::PyFileLikeObject;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::cmp::Ordering;

use chrono::{DateTime, Datelike, Timelike, Utc};
use widestring::U16String;

pub trait ReadSeek: Read + Seek {
    fn tell(&mut self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
}

impl<T: Read + Seek> ReadSeek for T {}

#[derive(Debug)]
pub enum FileOrFileLike {
    File(String),
    FileLike(PyFileLikeObject),
}

impl FileOrFileLike {
    pub fn from_pyobject(path_or_file_like: PyObject) -> PyResult<FileOrFileLike> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        // is a path
        if let Ok(string_ref) = path_or_file_like.cast_as::<PyString>(py) {
            return Ok(FileOrFileLike::File(
                string_ref.to_string_lossy().to_string(),
            ));
        }

        // We only need read + seek
        match PyFileLikeObject::with_requirements(path_or_file_like, true, false, true) {
            Ok(f) => Ok(FileOrFileLike::FileLike(f)),
            Err(e) => Err(e),
        }
    }
}

pub(crate) fn bytes_to_string(v: Vec<u8>, wide: bool) -> Option<String> {
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

fn nanos_to_micros_round_half_even(nanos: u32) -> u32 {
    let nanos_e7 = (nanos % 1000) / 100;
    let nanos_e6 = (nanos % 10000) / 1000;
    let mut micros = (nanos / 10000) * 10;
    match nanos_e7.cmp(&5) {
        Ordering::Greater => micros += nanos_e6 + 1,
        Ordering::Less => micros += nanos_e6,
        Ordering::Equal => micros += nanos_e6 + (nanos_e6 % 2),
    }
    micros
}

pub fn date_to_pyobject(date: &DateTime<Utc>) -> PyResult<PyObject> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    PyDateTime::new(
        py,
        date.year(),
        date.month() as u8,
        date.day() as u8,
        date.hour() as u8,
        date.minute() as u8,
        date.second() as u8,
        nanos_to_micros_round_half_even(date.timestamp_subsec_nanos()),
        None,
    )
    .map(|dt| dt.to_object(py))
}
