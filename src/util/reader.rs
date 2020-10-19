//reader.rs

use simple_error::SimpleError;
use std::fmt;
use log::error;

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

use crate::util::config::Config;
use crate::ese::ese_db::{esedb_file_header, ESEDB_FILE_SIGNATURE};
use crate::util::any_as_u32_slice;

pub fn load_db_file_header(config: &Config) -> Result<esedb_file_header, EseParserError> {
    let mut db_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(0))
        .map_err(EseParserError::Io)?;

    //debug!("db_file_header ({:X}): {:0X?}", mem::size_of::<esedb_file_header>(), db_file_header);

    assert_eq!(db_file_header.signature, ESEDB_FILE_SIGNATURE, "bad file_header.signature");

    fn calc_crc32(file_header: &&mut esedb_file_header) -> u32 {
        let vec32: &[u32] = unsafe{ any_as_u32_slice(& file_header) };
        vec32.iter().skip(1).fold(0x89abcdef as u32, |crc, &val| crc ^ val )
    }

    let stored_checksum = db_file_header.checksum;
    let checksum = calc_crc32(&&mut db_file_header);
    expect_eq!(stored_checksum, checksum, "wrong checksum");

    let backup_file_header = read_struct::<esedb_file_header, _>(&config.inp_file, SeekFrom::Start(db_file_header.page_size as u64))
        .map_err(EseParserError::Io)?;

    if db_file_header.format_revision == 0 {
        db_file_header.format_revision = backup_file_header.format_revision;
    }

    expect_eq!(db_file_header.format_revision, backup_file_header.format_revision, "mismatch in format revision");

    if db_file_header.page_size == 0 {
        db_file_header.page_size = backup_file_header.page_size;
    }

    expect_eq!(db_file_header.page_size, backup_file_header.page_size, "mismatch in page size");
    expect_eq!(db_file_header.format_version, 0x620, "unsupported format version");

    Ok(db_file_header)
}

use crate::ese::ese_db::{esedb_page_header};

pub fn load_page_header(config: &Config, db_file_header: &esedb_file_header, page_number: u64) -> Result<esedb_page_header, EseParserError> {
    let page_offset = (page_number + 1) * (db_file_header.page_size as u64);
    let mut db_page_header = read_struct::<esedb_page_header, _>(&config.inp_file, SeekFrom::Start(page_offset))
        .map_err(EseParserError::Io)?;
    let TODO_checksum = 0;

    Ok(db_page_header)
}