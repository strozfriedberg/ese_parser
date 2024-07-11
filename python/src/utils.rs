use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyDateTime, timezone_utc};
use pyo3::ToPyObject;
use pyo3::{PyObject, PyResult, Python};
use pyo3_file::PyFileLikeObject;
use std::cmp::Ordering;

use widestring::U16String;

#[derive(Debug)]
pub enum FileOrFileLike {
    File(String),
    FileLike(PyFileLikeObject),
}

impl FileOrFileLike {
    pub fn from_pyobject(path_or_file_like: PyObject) -> PyResult<FileOrFileLike> {
        Python::with_gil(|py| {
            // is a path
            if let Ok(s) = path_or_file_like.extract(py) {
                return Ok(FileOrFileLike::File(s));
            }

            // We only need read + seek
            match PyFileLikeObject::with_requirements(path_or_file_like, true, false, true) {
                Ok(f) => Ok(FileOrFileLike::FileLike(f)),
                Err(e) => Err(e),
            }
        })
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
    let nanos_e7 = (nanos % 1_000) / 100;
    let nanos_e6 = (nanos % 10_000) / 1000;
    let mut micros = (nanos / 10_000) * 10;
    match nanos_e7.cmp(&5) {
        Ordering::Greater => micros += nanos_e6 + 1,
        Ordering::Less => micros += nanos_e6,
        Ordering::Equal => micros += nanos_e6 + (nanos_e6 % 2),
    }
    micros
}

fn date_splitter(date: &DateTime<Utc>) -> PyResult<(i64, u32)> {
    let mut unix_time = date.timestamp();
    let mut micros = nanos_to_micros_round_half_even(date.timestamp_subsec_nanos());

    let inc_sec = micros / 1_000_000;
    micros %= 1_000_000;
    unix_time += inc_sec as i64;

    Ok((unix_time, micros))
}

pub fn date_to_pyobject(date: &DateTime<Utc>) -> PyResult<PyObject> {
    let (unix_time, micros) = date_splitter(date)?;

    let rounded_date = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(unix_time, micros * 1_000).ok_or(PyErr::new::<
            exceptions::PyTypeError,
            _,
        >(format!(
            "from_timestamp_opt({}, {}) failed",
            unix_time,
            micros * 1_000
        )))?,
        Utc,
    );

    Python::with_gil(|py| {
        PyDateTime::new(
            py,
            rounded_date.year(),
            rounded_date.month() as u8,
            rounded_date.day() as u8,
            rounded_date.hour() as u8,
            rounded_date.minute() as u8,
            rounded_date.second() as u8,
            rounded_date.timestamp_subsec_micros(),
            Some(timezone_utc(py))
        )
        .map(|dt| dt.to_object(py))
    })
}
