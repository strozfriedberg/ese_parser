//reader.rs

use std::fmt;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::mem::{self};
use std::slice;
//use simple_error::SimpleError;
use log::error;
use crate::ese::jet;

use crate::util::config::Config;
use crate::ese::ese_db;
use crate::ese::ese_db::{ESEDB_FILE_SIGNATURE, PageHeader, PageHeaderOld, PageHeader0x0b, PageHeader0x11, PageHeaderCommon, PageHeaderExt0x11, PageTag};
use crate::util::_any_as_slice;
//use std::mem::size_of;


#[derive(Debug)]
pub enum EseParserError {
    Io(io::Error),
    //Parse(SimpleError),
}
impl fmt::Display for EseParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EseParserError::Io(ref err) => write!(f, "IO error: {}", err),
            //EseParserError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

//https://stackoverflow.com/questions/38334994/how-to-read-a-c-struct-from-a-binary-file-in-rust
pub fn read_struct<T, P: AsRef<Path>>(path: P, file_offset: SeekFrom) -> io::Result<T> {
    let path = path.as_ref();
    let mut reader = BufReader::new(File::open(path)?);
    reader.seek(file_offset)?;
    let struct_size = mem::size_of::<T>();
    let mut r : T = unsafe { mem::zeroed() };
    unsafe {
        let buffer = slice::from_raw_parts_mut(&mut r as *mut _ as *mut u8, struct_size);
        reader.read_exact(buffer)?;
    }
    Ok(r)
}

// due "warning: function is never used: `load_db_file_header`" while main.rs:49
#[allow(dead_code)]
pub fn load_db_file_header(config: & Config) -> Result<ese_db::FileHeader, EseParserError> {
    let mut db_file_header = read_struct::<ese_db::FileHeader, _>(&config.inp_file, SeekFrom::Start(0))
        .map_err(EseParserError::Io)?;

    //debug!("db_file_header ({:X}): {:0X?}", mem::size_of::<esedb_file_header>(), db_file_header);

    assert_eq!(db_file_header.signature, ESEDB_FILE_SIGNATURE, "bad file_header.signature");

    fn calc_crc32(file_header: &&mut ese_db::FileHeader) -> u32 {
        let vec32: &[u32] = unsafe{ _any_as_slice::<u32, _>(& file_header) };
        vec32.iter().skip(1).fold(0x89abcdef, |crc, &val| crc ^ val )
    }

    let stored_checksum = db_file_header.checksum;
    let checksum = calc_crc32(&&mut db_file_header);
    expect_eq!(stored_checksum, checksum, "wrong checksum");

    let backup_file_header = read_struct::<ese_db::FileHeader, _>(&config.inp_file, SeekFrom::Start(db_file_header.page_size as u64))
        .map_err(EseParserError::Io)?;

    if db_file_header.format_revision.0 == 0 {
        db_file_header.format_revision = backup_file_header.format_revision;
    }

    expect_eq!(db_file_header.format_revision.0, backup_file_header.format_revision.0, "mismatch in format revision");

    if db_file_header.page_size == 0 {
        db_file_header.page_size = backup_file_header.page_size;
    }

    expect_eq!(db_file_header.page_size, backup_file_header.page_size, "mismatch in page size");
    expect_eq!(db_file_header.format_version.0, 0x620, "unsupported format version");

    Ok(db_file_header)
}

pub fn load_page_header(config: &Config, io_handle: &jet::IoHandle, page_number: u32) -> Result<PageHeader, EseParserError> {
    let page_offset = ((page_number + 1) * (io_handle.page_size)) as u64;
    let path = &config.inp_file;

    fn load<T>(path: &PathBuf, offs: u64) -> io::Result<T> {
        read_struct::<T, _> (path, SeekFrom::Start(offs))
    }

    if io_handle.format_revision.0 < 0x0000000b {
        let header = load::<PageHeaderOld>(path,page_offset).map_err(EseParserError::Io)?;
        let common = load::<PageHeaderCommon>(path,page_offset + mem::size_of_val(&header) as u64).map_err(EseParserError::Io)?;

        //let TODO_checksum = 0;
        Ok(PageHeader::old(header, common))
    }
    else if io_handle.format_revision.0 < 0x00000011 {
        let header = load::<PageHeader0x0b>(path,page_offset).map_err(EseParserError::Io)?;
        let common = load::<PageHeaderCommon>(path,page_offset + mem::size_of_val(&header) as u64).map_err(EseParserError::Io)?;

        //let TODO_checksum = 0;
        Ok(PageHeader::x0b(header, common))
    }
    else {
        let header = load::<PageHeader0x11>(path,page_offset).map_err(EseParserError::Io)?;
        let common = load::<PageHeaderCommon>(path,page_offset + mem::size_of_val(&header) as u64).map_err(EseParserError::Io)?;

        //let TODO_checksum = 0;
        if io_handle.page_size > 8 * 1024 {
            let offs = mem::size_of_val(&header) + mem::size_of_val(&common);
            let ext = load::<PageHeaderExt0x11>(path,page_offset + offs as u64).map_err(EseParserError::Io)?;

            Ok(PageHeader::x11_ext(header, common, ext))
        }
        else {
            Ok(PageHeader::x11(header, common))
        }
    }
}

pub fn load_page_tags(config: &Config, io_handle: &jet::IoHandle, db_page: &jet::DbPage) -> Result<Vec<PageTag>, EseParserError> {
    let page_offset = (db_page.page_number + 1) * io_handle.page_size;
    let mut tags_offset = (page_offset + io_handle.page_size - 4) as u64;
    let path = &config.inp_file;
    let mut tags = Vec::<PageTag>::new();

    for _i in 0..db_page.get_available_page_tag() {
        if io_handle.format_revision.0 >= 0x00000011 && io_handle.page_size > 8 * 1024 {
            let tag = read_struct::<ese_db::PageTag0x11, _> (path, SeekFrom::Start(tags_offset)).map_err(EseParserError::Io)?;
            tags.push(PageTag::x11(tag));
        }
        else {
            let tag = read_struct::<ese_db::PageTagOld, _> (path, SeekFrom::Start(tags_offset)).map_err(EseParserError::Io)?;
            tags.push(PageTag::old(tag));
        }
        tags_offset -= 4;
    }

    Ok(tags)
}