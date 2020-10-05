//db_file_header.rs
#![allow( non_camel_case_types, non_upper_case_globals, )]
use crate::ese::ctypes::{ uint8_t, uint32_t/*, uint64_t*/ };
use crate::ese::jet;

pub static  esedb_file_signature: uint32_t = 0x89abcdef;

pub type esedb_file_header_t = esedb_file_header;
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct esedb_file_header {
    pub checksum: uint32_t,
    pub signature: uint32_t,
    pub format_version: uint32_t,
    pub file_type: uint32_t,
    pub database_time: [uint8_t; 8],
    pub database_signature: jet::Signature,
    pub database_state: jet::DbState,
    pub consistent_postition: [uint8_t; 8],
    pub consistent_time: [uint8_t; 8],
    pub attach_time: [uint8_t; 8],
    pub attach_postition: [uint8_t; 8],
    pub detach_time: [uint8_t; 8],
    pub detach_postition: [uint8_t; 8],
    pub unknown1: uint32_t,
    pub log_signature: jet::Signature,
    pub previous_full_backup: [uint8_t; 24],
    pub previous_incremental_backup: [uint8_t; 24],
    pub current_full_backup: [uint8_t; 24],
    pub shadowing_disabled: uint32_t,
    pub last_object_identifier: uint32_t,
    pub index_update_major_version: uint32_t,
    pub index_update_minor_version: uint32_t,
    pub index_update_build_number: uint32_t,
    pub index_update_service_pack_number: uint32_t,
    pub format_revision: uint32_t,
    pub page_size: uint32_t,
    pub repair_count: uint32_t,
    pub repair_time: [uint8_t; 8],
    pub unknown2: jet::Signature,
    pub scrub_database_time: [uint8_t; 8],
    pub scrub_time: [uint8_t; 8],
    pub required_log: [uint8_t; 8],
    pub upgrade_exchange5_format: uint32_t,
    pub upgrade_free_pages: uint32_t,
    pub upgrade_space_map_pages: uint32_t,
    pub current_shadow_volume_backup: [uint8_t; 24],
    pub creation_format_version: uint32_t,
    pub creation_format_revision: uint32_t,
    pub unknown3: [uint8_t; 16],
    pub old_repair_count: uint32_t,
    pub ecc_fix_success_count: uint32_t,
    pub ecc_fix_success_time: [uint8_t; 8],
    pub old_ecc_fix_success_count: uint32_t,
    pub ecc_fix_error_count: uint32_t,
    pub ecc_fix_error_time: [uint8_t; 8],
    pub old_ecc_fix_error_count: uint32_t,
    pub bad_checksum_error_count: uint32_t,
    pub bad_checksum_error_time: [uint8_t; 8],
    pub old_bad_checksum_error_count: uint32_t,
    pub committed_log: uint32_t,
    pub previous_shadow_volume_backup: [uint8_t; 24],
    pub previous_differential_backup: [uint8_t; 24],
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

