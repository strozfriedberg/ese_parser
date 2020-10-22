//ese_db
#![allow( non_camel_case_types, dead_code )]
use std::fmt;
use crate::ese::jet;
use winapi::_core::fmt::{Debug, Formatter};

use jet::{ uint8_t, uint16_t, uint32_t, uint64_t };

pub static ESEDB_FILE_SIGNATURE: uint32_t = 0x89abcdef;
pub static ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT: uint32_t = 0x0b;
pub static ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER: uint32_t = 0x11;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct esedb_file_header {
    pub checksum: uint32_t,
    pub signature: uint32_t,
    pub format_version: uint32_t,
    pub file_type: uint32_t,
    pub database_time: jet::DbTime,
    pub database_signature: jet::Signature,
    pub database_state: jet::DbState,
    pub consistent_postition: jet::LgPos,
    pub consistent_time: jet::DateTime,
    pub attach_time: jet::DateTime,
    pub attach_postition: jet::LgPos,
    pub detach_time: jet::DateTime,
    pub detach_postition: jet::LgPos,
    pub dbid: uint32_t,
    pub log_signature: jet::Signature,
    pub previous_full_backup: jet::BackupInfo,
    pub previous_incremental_backup: jet::BackupInfo,
    pub current_full_backup: jet::BackupInfo,
    pub shadowing_disabled: uint32_t,
    pub last_object_identifier: uint32_t,
    pub index_update_major_version: uint32_t,
    pub index_update_minor_version: uint32_t,
    pub index_update_build_number: uint32_t,
    pub index_update_service_pack_number: uint32_t,
    pub format_revision: uint32_t,
    pub page_size: uint32_t,
    pub repair_count: uint32_t,
    pub repair_time: jet::DateTime,
    pub unknown2: jet::Signature,
    pub scrub_database_time: jet::DateTime,
    pub scrub_time: jet::DateTime,
    pub required_log: [uint8_t; 8],
    pub upgrade_exchange5_format: uint32_t,
    pub upgrade_free_pages: uint32_t,
    pub upgrade_space_map_pages: uint32_t,
    pub current_shadow_volume_backup: jet::BackupInfo,
    pub creation_format_version: uint32_t,
    pub creation_format_revision: uint32_t,
    pub unknown3: [uint8_t; 16],
    pub old_repair_count: uint32_t,
    pub ecc_fix_success_count: uint32_t,
    pub ecc_fix_success_time: jet::DateTime,
    pub old_ecc_fix_success_count: uint32_t,
    pub ecc_fix_error_count: uint32_t,
    pub ecc_fix_error_time: jet::DateTime,
    pub old_ecc_fix_error_count: uint32_t,
    pub bad_checksum_error_count: uint32_t,
    pub bad_checksum_error_time: jet::DateTime,
    pub old_bad_checksum_error_count: uint32_t,
    pub committed_log: uint32_t,
    pub previous_shadow_volume_backup: jet::BackupInfo,
    pub previous_differential_backup: jet::BackupInfo,
    pub unknown4_1: [uint8_t; 20],
    pub unknown4_2: [uint8_t; 40 - 20],
    pub nls_major_version: uint32_t,
    pub nls_minor_version: uint32_t,
    pub unknown5_1: [uint8_t; 32],
    pub unknown5_2: [uint8_t; 32],
    pub unknown5_3: [uint8_t; 32],
    pub unknown5_4: [uint8_t; 32],
    pub unknown5_5: [uint8_t; 148 - 4 * 32],
    pub unknown_flags: uint32_t,
    pub unknown_val: uint32_t,
}

pub type LIBESEDB_PAGE_FLAGS = libc::c_uint;
pub const LIBESEDB_PAGE_FLAG_0X10000: LIBESEDB_PAGE_FLAGS = 65536;
pub const LIBESEDB_PAGE_FLAG_0X8000: LIBESEDB_PAGE_FLAGS = 32768;
pub const LIBESEDB_PAGE_FLAG_IS_SCRUBBED: LIBESEDB_PAGE_FLAGS = 16384;
pub const LIBESEDB_PAGE_FLAG_IS_NEW_RECORD_FORMAT: LIBESEDB_PAGE_FLAGS = 8192;
pub const LIBESEDB_PAGE_FLAG_0X0800: LIBESEDB_PAGE_FLAGS = 2048;
pub const LIBESEDB_PAGE_FLAG_0X0400: LIBESEDB_PAGE_FLAGS = 1024;
pub const LIBESEDB_PAGE_FLAG_IS_LONG_VALUE: LIBESEDB_PAGE_FLAGS = 128;
pub const LIBESEDB_PAGE_FLAG_IS_INDEX: LIBESEDB_PAGE_FLAGS = 64;
pub const LIBESEDB_PAGE_FLAG_IS_SPACE_TREE: LIBESEDB_PAGE_FLAGS = 32;
pub const LIBESEDB_PAGE_FLAG_IS_EMPTY: LIBESEDB_PAGE_FLAGS = 8;
pub const LIBESEDB_PAGE_FLAG_IS_PARENT: LIBESEDB_PAGE_FLAGS = 4;
pub const LIBESEDB_PAGE_FLAG_IS_LEAF: LIBESEDB_PAGE_FLAGS = 2;
pub const LIBESEDB_PAGE_FLAG_IS_ROOT: LIBESEDB_PAGE_FLAGS = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct PageHeaderOld {
    pub xor_checksum: uint32_t,
    pub page_number: uint32_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct PageHeader0x0b {
    pub xor_checksum: uint32_t,
    pub ecc_checksum: uint32_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct PageHeader0x11 {
    pub checksum: uint64_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct PageHeaderCommon {
        specific: [uint8_t; 8],
    pub database_modification_time: jet::DateTime,
    pub previous_page: uint32_t,
    pub next_page: uint32_t,
    pub father_data_page_object_identifier: uint32_t,
    pub available_data_size: uint16_t,
    pub available_uncommitted_data_size: uint16_t,
    pub available_data_offset: uint16_t,
    pub available_page_tag: uint16_t,
    pub page_flags: uint32_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
union esedb_page_header {
    pub page_header_old: PageHeaderOld,
    pub page_header_0x0b: PageHeader0x0b,
    pub page_header_0x11: PageHeader0x11,
    pub page_header_common: PageHeaderCommon,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct  PageHeaderExt0x11 {
    pub checksum1: uint64_t,
    pub checksum2: uint64_t,
    pub checksum3: uint64_t,
    pub page_number: uint64_t,
    pub unknown: uint64_t,
}

pub struct EsePageHeader {
    db_file_header: esedb_file_header,
    page_header: esedb_page_header,
}

use crate::util::config::Config;
use crate::util::reader::{EseParserError, read_struct};
use std::io::SeekFrom;

fn load_page_header(config: &Config, db_file_header: &esedb_file_header, page_number: u64) -> Result<esedb_page_header, EseParserError> {
    let page_offset = (page_number + 1) * (db_file_header.page_size as u64);
    let db_page_header = read_struct::<esedb_page_header, _>(&config.inp_file, SeekFrom::Start(page_offset))
        .map_err(EseParserError::Io)?;
    let TODO_checksum = 0;

    Ok(db_page_header)
}

impl EsePageHeader {
    pub fn new(config: &Config, db_file_header: &esedb_file_header, page_number: u64) -> EsePageHeader {
        let page_header = load_page_header(config, db_file_header, page_number).unwrap();

        EsePageHeader{ db_file_header: db_file_header.clone(),
                        page_header: page_header }
    }
}

impl Debug for EsePageHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            if self.db_file_header.format_revision < 0x0000000b {
                write!(f, "{:?} {:?}", self.page_header.page_header_old, self.page_header.page_header_common)
            }
            else if self.db_file_header.format_revision < 0x00000011 {
                write!(f, "{:?} {:?}", self.page_header.page_header_0x0b, self.page_header.page_header_common)
            }
            else {
                write!(f, "{:?} {:?}", self.page_header.page_header_0x11, self.page_header.page_header_common)
            }
        }
    }
}

