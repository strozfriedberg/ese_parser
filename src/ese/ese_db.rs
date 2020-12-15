//ese_db
#![allow( non_camel_case_types, dead_code )]
use crate::ese::jet;
use winapi::_core::fmt::Debug;

use jet::{ uint8_t, uint16_t, uint32_t, uint64_t, PageFlags };

pub const ESEDB_FILE_SIGNATURE: uint32_t = 0x89abcdef;
pub const ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT: uint32_t = 0x0b;
pub const ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER: uint32_t = 0x11;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct FileHeader {
    pub checksum: uint32_t,
    pub signature: uint32_t,
    pub format_version: jet::FormatVersion,
    pub file_type: jet::FileType,
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
    pub format_revision: jet::FormatRevision,
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeaderOld {
    pub xor_checksum: uint32_t,
    pub page_number: uint32_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeader0x0b {
    pub xor_checksum: uint32_t,
    pub ecc_checksum: uint32_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeader0x11 {
    pub checksum: uint64_t,
    pub database_modification_time: jet::DateTime,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeaderCommon {
    pub database_modification_time: jet::DateTime,
    pub previous_page: uint32_t,
    pub next_page: uint32_t,
    pub father_data_page_object_identifier: uint32_t,
    pub available_data_size: uint16_t,
    pub available_uncommitted_data_size: uint16_t,
    pub available_data_offset: uint16_t,
    pub available_page_tag: uint16_t,
    pub page_flags: PageFlags,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct  PageHeaderExt0x11 {
    pub checksum1: uint64_t,
    pub checksum2: uint64_t,
    pub checksum3: uint64_t,
    pub page_number: uint64_t,
    pub unknown: uint64_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum page_header {
    old (PageHeaderOld, PageHeaderCommon),
    x0b (PageHeader0x0b, PageHeaderCommon),
    x11 (PageHeader0x11, PageHeaderCommon),
    x11_ext (PageHeader0x11, PageHeaderCommon, PageHeaderExt0x11),
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageValue {
    pub data: *mut uint8_t,
    pub size: uint16_t,
    pub offset: uint16_t,
    pub flags: uint8_t,
}
