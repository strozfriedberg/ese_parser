//jet.rs
#![allow( non_camel_case_types, dead_code )]
use crate::ese::ese_db;

use bitflags::bitflags;
use std::{mem, fmt};
use std::rc::Rc;
use chrono::TimeZone;
use chrono::naive::{NaiveDate, NaiveTime};
use winapi::um::timezoneapi::{GetTimeZoneInformation/*, FileTimeToSystemTime*/};
use strum::Display;

use crate::util::config::Config;
use crate::util::reader::{load_page_header, load_page_tags};
//use std::path::Display;
use bitflags::_core::fmt::Formatter;
use crate::ese::ese_db::{PageHeader, PageTag};

pub type uint8_t = ::std::os::raw::c_uchar;
pub type uint16_t = ::std::os::raw::c_short;
pub type uint32_t = ::std::os::raw::c_ulong;
pub type uint64_t = ::std::os::raw::c_ulonglong;
pub type off64_t = ::std::os::raw::c_longlong;
pub type size64_t = uint64_t;

type OsDateTime = chrono::DateTime<chrono::Utc>;

#[derive(Copy, Clone, Default, Debug)]
pub struct FormatVersion(pub(crate) uint32_t);
impl std::fmt::Display for FormatVersion { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0)  } }
#[derive(Copy, Clone, Default, Debug)]
pub struct FormatRevision(pub(crate) uint32_t);
impl std::fmt::Display for FormatRevision { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0)  } }

bitflags! {
    pub struct PageFlags: uint32_t {
        const UNKNOWN_8000          = 0b1000000000000000;
        const IS_SCRUBBED           = 0b0100000000000000;
        const IS_NEW_RECORD_FORMAT  = 0b0010000000000000;
        const UNKNOWN_1000          = 0b0001000000000000;
        const UNKNOWN_800           = 0b0000100000000000;
        const UNKNOWN_400           = 0b0000010000000000;
        const UNKNOWN_200           = 0b0000001000000000;
        const UNKNOWN_100           = 0b0000000100000000;
        const IS_LONG_VALUE         = 0b0000000010000000;
        const IS_INDEX              = 0b0000000001000000;
        const IS_SPACE_TREE         = 0b0000000000100000;
        const UNKNOWN_10            = 0b0000000000010000;
        const IS_EMPTY              = 0b0000000000001000;
        const IS_PARENT             = 0b0000000000000100;
        const IS_LEAF               = 0b0000000000000010;
        const IS_ROOT               = 0b0000000000000001;
    }
}

#[derive(Copy, Clone, Debug)]
pub enum FixedPageNumber {
    Database        = 1,
    Catalog         = 4,
    CatalogBackup   = 24,
}

#[derive(Copy, Clone, Debug)]
pub enum FixedFDPNumber {
    Database        = 1,
    Catalog         = 2,
    CatalogBackup   = 3,
}

#[derive(Copy, Clone, Debug)]
pub enum CatalogType {
    Table           = 1,
    Column          = 2,
    Index           = 3,
    LongValue       = 4,
    Callback        = 5,
}

#[derive(Copy, Clone, Debug)]
pub enum ColumnType {
    Nil             = 0,
    Bit             = 1,
    UnsignedByte    = 2,
    Short           = 3,
    Long            = 4,
    Currency        = 5,
    IEEESingle      = 6,
    IEEEDouble      = 7,
    DateTime        = 8,
    Binary          = 9,
    Text            = 10,
    LongBinary      = 11,
    LongText        = 12,
    SLV             = 13,
    UnsignedLong    = 14,
    LongLong        = 15,
    GUID            = 16,
    UnsignedShort   = 17,
    Max             = 18,
}

bitflags! {
    pub struct TaggedDataTypeFlag : uint16_t {
        const VARIABLE_SIZE = 0b00000001;
        const COMPRESSED    = 0b00000010;
        const STORED        = 0b00000100;
        const MULTI_VALUE   = 0b00001000;
        const WHO_KNOWS     = 0b00010000;
    }
}

#[derive(Copy, Clone, Display, Debug)]
#[repr(u32)]
pub enum DbState {
    impossible      = 0,
    JustCreated     = 1,
    DirtyShutdown   = 2,
    CleanShutdown   = 3,
    BeingConverted  = 4,
    ForceDetach     = 5
}

impl Default for DbState {
    fn default() -> Self { DbState::impossible }
}


#[derive(Copy, Clone, Display, Debug)]
#[repr(u32)]
pub enum FileType {
    Database = 0,
    StreamingFile = 1,
}

impl Default for FileType {
    fn default() -> Self { FileType::Database }
}

#[derive(Copy, Clone, Default, Debug)]
#[repr(C)]
pub struct DbTime {
    pub hours: uint16_t,
    pub minutes: uint16_t,
    pub seconds: uint16_t,
    pub padding: uint16_t,
}

impl fmt::Display for DbTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(t) = NaiveTime::from_hms_opt(self.hours as u32, self.minutes as u32, self.seconds as u32) {
            write!(f, "{}", t)
        }
        else {
            write!(f, "Bad DbTime: {:?}", self)
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
#[repr(C)]
pub struct DateTime {
    pub seconds: uint8_t,
    pub minutes: uint8_t,
    pub hours: uint8_t,
    pub day: uint8_t,
    pub month: uint8_t,
    pub year: uint8_t,
    pub time_is_utc: uint8_t,
    pub os_snapshot: uint8_t,
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.year > 0 {
            let ndt = NaiveDate::from_ymd(self.year as i32 + 1900, self.month as u32, self.day as u32)
                .and_hms(self.hours as u32, self.minutes as u32, self.seconds as u32);
            let offset = if self.time_is_utc != 0 {
                0
            }
            else {
                unsafe{
                    let mut tz = mem::zeroed();
                    GetTimeZoneInformation(&mut tz);
                    -60 * (tz.Bias + tz.StandardBias)
                }
            };
            let dt: OsDateTime = OsDateTime::from(chrono::FixedOffset::east(offset).from_local_datetime(&ndt).unwrap());

            write!(f, "{}", dt)
        } else {
            write!(f, "")
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
#[repr(C)]
pub struct Signature {
    pub random: uint32_t,
    pub logtime_create: DateTime,
    pub computer_name: [uint8_t; 16],
}

#[repr(C, packed)]
#[derive(Debug, Copy, Default, Clone)]
pub struct LgPos {
    pub ib: ::std::os::raw::c_ushort,
    pub isec: ::std::os::raw::c_ushort,
    pub l_generation: ::std::os::raw::c_long,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Default, Clone)]
pub struct BackupInfo {
    pub lg_pos_mark: LgPos,
    pub bk_logtime_mark: DateTime,
    pub gen_low: ::std::os::raw::c_ulong,
    pub gen_high: ::std::os::raw::c_ulong,
}

#[derive(Debug)]
pub struct DbFile {
    file_header: ese_db::FileHeader,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IoHandle {
    pub file_type: FileType,
    pub format_version: FormatVersion,
    pub format_revision: FormatRevision,
    pub creation_format_version: uint32_t,
    pub creation_format_revision: uint32_t,
    pub pages_data_offset: off64_t,
    pub pages_data_size: size64_t,
    pub page_size: uint32_t,
    pub last_page_number: uint32_t,
    pub ascii_codepage: libc::c_int,
    pub abort: libc::c_int,
}

impl IoHandle {
    pub fn new(db_file_header: &ese_db::FileHeader) -> IoHandle {
        let pages_data_offset: off64_t = (db_file_header.page_size * 2) as off64_t;

        IoHandle {
            file_type: db_file_header.file_type,
            format_version: db_file_header.format_version,
            format_revision: db_file_header.format_revision,
            creation_format_version: db_file_header.creation_format_version,
            creation_format_revision: db_file_header.creation_format_revision,
            page_size: db_file_header.page_size,

            pages_data_offset: pages_data_offset,
            pages_data_size: pages_data_offset as u64,
            last_page_number: (pages_data_offset / db_file_header.page_size as i64) as u32,

            ascii_codepage: 0,
            abort: 0
        }
    }
}

pub struct DbPage {
    pub page_number: uint32_t,
    pub page_header: ese_db::PageHeader,
    pub page_tags: Vec<ese_db::PageTag>,
}

impl DbPage {
    pub fn new(config: &Config, io_handle: &IoHandle, page_number: u32) -> DbPage {
        let page_header = load_page_header(config, io_handle, page_number).unwrap();
        let mut db_page = DbPage{page_number: page_number, page_header: page_header, page_tags: vec![] };

        db_page.page_tags = load_page_tags(config, io_handle, &db_page).unwrap();
        db_page
    }

    pub fn get_available_page_tag(&self) -> usize {
        match self.page_header {
            PageHeader::old(_, common) => common.available_page_tag as usize,
            PageHeader::x0b(_, common) => common.available_page_tag as usize,
            PageHeader::x11(_, common) => common.available_page_tag as usize,
            PageHeader::x11_ext(_, common, _) => common.available_page_tag as usize,
        }
    }
}

impl PageTag {
    pub fn size(&self) -> u32 {
        match self {
            PageTag::old(x) => x.size(),
            PageTag::x11(x) => x.size(),
        }
    }

    pub fn offset(&self) -> u32 {
        match self {
            PageTag::old(x) => x.offset(),
            PageTag::x11(x) => x.offset(),
        }
    }
}


#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub father_data_page_number: uint32_t,
    pub column_type: uint32_t,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CatalogDefinition {
    pub father_data_page_object_identifier: uint32_t,
    pub type_0: uint16_t,
    pub identifier: uint32_t,
    pub c2rust_unnamed: C2RustUnnamed,
    pub size: uint32_t,
    pub codepage: uint32_t,
    pub lcmap_flags: uint32_t,
    pub name: *mut uint8_t,
    pub name_size: uint32_t,
    pub template_name: *mut uint8_t,
    pub template_name_size: uint32_t,
    pub default_value: *mut uint8_t,
    pub default_value_size: uint32_t,
}

#[derive(Clone)]
#[repr(C)]
pub struct TableDefinition {
    pub table_catalog_definition: Rc<CatalogDefinition>,
    pub long_value_catalog_definition: Rc<CatalogDefinition>,
    pub callback_catalog_definition: Rc<CatalogDefinition>,
    //pub column_catalog_definition_array: *mut libcdata_array_t,
    //pub index_catalog_definition_array: *mut libcdata_array_t,
}

pub struct PageTree {
    //pub io_handle: *mut libesedb_io_handle_t,
    pub object_identifier: uint32_t,
    pub root_page_number: uint32_t,
    pub table_definition: TableDefinition,
    pub template_table_definition: TableDefinition,
    // pub pages_vector: *mut libfdata_vector_t,
    // pub pages_cache: *mut libfcache_cache_t,
    // pub root_page_header: *mut libesedb_root_page_header_t,
    // pub leaf_page_descriptors_tree: *mut libcdata_btree_t,
    pub number_of_leaf_values: uint32_t,
}

pub fn revision_to_string(version: FormatVersion, revision: FormatRevision) -> String {
    let s = match (version.0, revision.0) {
                    (0x00000620, 0x00000000) => "Original operating system Beta format (April 22, 1997)",
                    (0x00000620, 0x00000001) => "Add columns in the catalog for conditional indexing and OLD (May 29, 1997)",
                    (0x00000620, 0x00000002) => "Add the fLocalizedText flag in IDB (July 5, 1997), Revert revision in order for ESE97 to remain forward-compatible (January 28, 1998)",
                    (0x00000620, 0x00000003) => "Add SPLIT_BUFFER to space tree root pages (October 30, 1997), Add new tagged columns to catalog (\"CallbackData\" and \"CallbackDependencies\")",
                    (0x00000620, 0x00000004) => "Super Long Value (SLV) support: signSLV, fSLVExists in db header (May 5, 1998)",
                    (0x00000620, 0x00000005) => "New SLV space tree (May 29, 1998)",
                    (0x00000620, 0x00000006) => "SLV space map (October 12, 1998)",
                    (0x00000620, 0x00000007) => "4-byte IDXSEG (December 10, 1998)",
                    (0x00000620, 0x00000008) => "New template column format (January 25, 1999)",
                    (0x00000620, 0x00000009) => "Sorted template columns (July 24, 1999). Used in Windows XP SP3",
                    (0x00000620, 0x0000000b) => "Contains the page header with the ECC checksum Used in Exchange",
                    (0x00000620, 0x0000000c) => "Used in Windows Vista (SP0)",
                    (0x00000620, 0x00000011) => "Support for 2 KiB, 16 KiB and 32 KiB pages. Extended page header with additional ECC checksums. Column compression. Space hints. Used in Windows 7 (SP0)",
                    (0x00000620, 0x00000014) => "Used in Exchange 2013 and Active Directory 2016",
                    (0x00000623, 0x00000000) => "New Space Manager (May 15, 1999)",
                    _ => "Unknown",
                };
    format!("{:#x}, {:#x}: {}", version.0, revision.0, s)
}

/*
use winapi::um::minwinbase::{SYSTEMTIME, LPSYSTEMTIME};
use winapi::shared::minwindef::FILETIME;
pub type FileTime = FILETIME;

pub fn filetime_to_string(ft: FileTime) -> String {
    let mut st: SYSTEMTIME = unsafe {mem::zeroed()};
    let p_st: LPSYSTEMTIME = &mut st;

    if unsafe {FileTimeToSystemTime(&ft, p_st )} != 0 {
        let ndt = NaiveDate::from_ymd(st.wYear as i32, st.wMonth as u32, st.wDay as u32)
            .and_hms_milli(st.wHour as u32, st.wMinute as u32, st.wSecond as u32, st.wMilliseconds as u32);
        let dt = OsDateTime::from_utc(ndt, chrono::Utc);
        dt.to_string()
    }
    else {
        "".to_string()
    }
}
 */

