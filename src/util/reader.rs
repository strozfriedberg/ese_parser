//reader.rs

use simple_error::SimpleError;
use std::fmt;

#[derive(Debug)]
pub enum EseParserError {
    Io(io::Error),
    Parse(SimpleError),
}
impl fmt::Display for EseParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EseParserError::Io(ref err) => write!(f, "IO error: {}", err),
            EseParserError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

//https://stackoverflow.com/questions/38334994/how-to-read-a-c-struct-from-a-binary-file-in-rust
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::mem::{self};
use std::slice;

pub fn read_struct<T, P: AsRef<Path>>(path: P, file_offset: SeekFrom) -> io::Result<T> {
    let path = path.as_ref();
    let struct_size = mem::size_of::<T>();
    let mut reader = BufReader::new(File::open(path)?);
    reader.seek(file_offset)?;
    let mut r : T = unsafe { mem::zeroed() };
    unsafe {
        let buffer = slice::from_raw_parts_mut(&mut r as *mut _ as *mut u8, struct_size);
        reader.read_exact(buffer)?;
    }
    Ok(r)
}

