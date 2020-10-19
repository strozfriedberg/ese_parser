//ese_db
#![allow( non_camel_case_types, dead_code )]
use std::fmt;
use crate::ese::jet;
use winapi::_core::fmt::{Debug, Formatter};

type uint8_t = ::std::os::raw::c_uchar;
type uint32_t = ::std::os::raw::c_ulong;

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
pub const LIBESEDB_PAGE_FLAG_0x10000: LIBESEDB_PAGE_FLAGS = 65536;
pub const LIBESEDB_PAGE_FLAG_0x8000: LIBESEDB_PAGE_FLAGS = 32768;
pub const LIBESEDB_PAGE_FLAG_IS_SCRUBBED: LIBESEDB_PAGE_FLAGS = 16384;
pub const LIBESEDB_PAGE_FLAG_IS_NEW_RECORD_FORMAT: LIBESEDB_PAGE_FLAGS = 8192;
pub const LIBESEDB_PAGE_FLAG_0x0800: LIBESEDB_PAGE_FLAGS = 2048;
pub const LIBESEDB_PAGE_FLAG_0x0400: LIBESEDB_PAGE_FLAGS = 1024;
pub const LIBESEDB_PAGE_FLAG_IS_LONG_VALUE: LIBESEDB_PAGE_FLAGS = 128;
pub const LIBESEDB_PAGE_FLAG_IS_INDEX: LIBESEDB_PAGE_FLAGS = 64;
pub const LIBESEDB_PAGE_FLAG_IS_SPACE_TREE: LIBESEDB_PAGE_FLAGS = 32;
pub const LIBESEDB_PAGE_FLAG_IS_EMPTY: LIBESEDB_PAGE_FLAGS = 8;
pub const LIBESEDB_PAGE_FLAG_IS_PARENT: LIBESEDB_PAGE_FLAGS = 4;
pub const LIBESEDB_PAGE_FLAG_IS_LEAF: LIBESEDB_PAGE_FLAGS = 2;
pub const LIBESEDB_PAGE_FLAG_IS_ROOT: LIBESEDB_PAGE_FLAGS = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct esedb_page_header {
    pub xor_checksum: [u8; 4usize],
    pub __page_number_ecc_checksum: esedb_page_header__page_number_ecc_checksum,
    pub database_modification_time: [u8; 8usize],
    pub previous_page: [u8; 4usize],
    pub next_page: [u8; 4usize],
    pub father_data_page_object_identifier: [u8; 4usize],
    pub available_data_size: [u8; 2usize],
    pub available_uncommitted_data_size: [u8; 2usize],
    pub available_data_offset: [u8; 2usize],
    pub available_page_tag: [u8; 2usize],
    pub page_flags: [u8; 4usize],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union esedb_page_header__page_number_ecc_checksum {
    pub page_number: [u8; 4usize],
    pub ecc_checksum: [u8; 4usize],
    _bindgen_union_align: [u8; 4usize],
}

impl Debug for esedb_page_header__page_number_ecc_checksum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("esedb_page_header__page_number_ecc_checksum")
            .field("page_number of ecc_checksum", unsafe {&self.page_number})
            .finish()
    }
}