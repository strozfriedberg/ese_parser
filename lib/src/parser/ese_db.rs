#![allow(non_camel_case_types)]

use crate::impl_read_struct;
use crate::impl_read_struct_buffer;
use crate::parser::jet;
use crate::parser::jet::*;
use crate::parser::reader::{ReadSeek, mark_used};
use nom::number::complete::le_u32;
use nom_derive::*;

pub const ESEDB_FILE_SIGNATURE: uint32_t = 0x89abcdef;
pub const ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT: uint32_t = 0x0b;
pub const ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER: uint32_t = 0x11;

#[derive(Copy, Clone, Default, Debug, Nom)]
#[repr(C)]
pub struct FileHeader {
    pub checksum: uint32_t,
    pub signature: uint32_t,
    #[nom(Parse = "{ |i| jet::FormatVersion::parse_le(i) }")]
    pub format_version: jet::FormatVersion,
    #[nom(Parse = "{ |i| jet::FileType::parse_le(i) }")]
    pub file_type: jet::FileType,
    #[nom(Parse = "{ |i| jet::DbTime::parse_le(i) }")]
    pub database_time: jet::DbTime,
    #[nom(Parse = "{ |i| jet::Signature::parse_le(i) }")]
    pub database_signature: jet::Signature,
    #[nom(Parse = "{ |i| jet::DbState::parse_le(i) }")]
    pub database_state: jet::DbState,
    #[nom(Parse = "{ |i| jet::LgPos::parse_le(i) }")]
    pub consistent_postition: jet::LgPos,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub consistent_time: jet::DateTime,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub attach_time: jet::DateTime,
    #[nom(Parse = "{ |i| jet::LgPos::parse_le(i) }")]
    pub attach_postition: jet::LgPos,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub detach_time: jet::DateTime,
    #[nom(Parse = "{ |i| jet::LgPos::parse_le(i) }")]
    pub detach_postition: jet::LgPos,
    pub dbid: uint32_t,
    #[nom(Parse = "{ |i| jet::Signature::parse_le(i) }")]
    pub log_signature: jet::Signature,
    #[nom(Parse = "{ |i| jet::BackupInfo::parse_le(i) }")]
    pub previous_full_backup: jet::BackupInfo,
    #[nom(Parse = "{ |i| jet::BackupInfo::parse_le(i) }")]
    pub previous_incremental_backup: jet::BackupInfo,
    #[nom(Parse = "{ |i| jet::BackupInfo::parse_le(i) }")]
    pub current_full_backup: jet::BackupInfo,
    pub shadowing_disabled: uint32_t,
    pub last_object_identifier: uint32_t,
    pub index_update_major_version: uint32_t,
    pub index_update_minor_version: uint32_t,
    pub index_update_build_number: uint32_t,
    pub index_update_service_pack_number: uint32_t,
    #[nom(Parse = "le_u32")]
    pub format_revision: jet::FormatRevision,
    pub page_size: uint32_t,
    pub repair_count: uint32_t,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub repair_time: jet::DateTime,
    #[nom(Parse = "{ |i| jet::Signature::parse_le(i) }")]
    pub unknown2: jet::Signature,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub scrub_database_time: jet::DateTime,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub scrub_time: jet::DateTime,
    pub required_log: [uint8_t; 8],
    pub upgrade_exchange5_format: uint32_t,
    pub upgrade_free_pages: uint32_t,
    pub upgrade_space_map_pages: uint32_t,
    #[nom(Parse = "{ |i| jet::BackupInfo::parse_le(i) }")]
    pub current_shadow_volume_backup: jet::BackupInfo,
    pub creation_format_version: uint32_t,
    pub creation_format_revision: uint32_t,
    pub unknown3: [uint8_t; 16],
    pub old_repair_count: uint32_t,
    pub ecc_fix_success_count: uint32_t,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub ecc_fix_success_time: jet::DateTime,
    pub old_ecc_fix_success_count: uint32_t,
    pub ecc_fix_error_count: uint32_t,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub ecc_fix_error_time: jet::DateTime,
    pub old_ecc_fix_error_count: uint32_t,
    pub bad_checksum_error_count: uint32_t,
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub bad_checksum_error_time: jet::DateTime,
    pub old_bad_checksum_error_count: uint32_t,
    pub committed_log: uint32_t,
    #[nom(Parse = "{ |i| jet::BackupInfo::parse_le(i) }")]
    pub previous_shadow_volume_backup: jet::BackupInfo,
    #[nom(Parse = "{ |i| jet::BackupInfo::parse_le(i) }")]
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
impl_read_struct_buffer!(FileHeader);

#[repr(packed)]
#[derive(Copy, Clone, Debug, Nom)]
pub struct PageHeaderOld {
    pub xor_checksum: uint32_t,
    pub page_number: uint32_t,
}
impl_read_struct!(PageHeaderOld);

#[repr(packed)]
#[derive(Copy, Clone, Debug, Nom)]
pub struct PageHeader0x0b {
    pub xor_checksum: uint32_t,
    pub ecc_checksum: uint32_t,
}
impl_read_struct!(PageHeader0x0b);

#[repr(packed)]
#[derive(Copy, Clone, Debug, Nom)]
pub struct PageHeader0x11 {
    pub checksum: uint64_t,
}
impl_read_struct!(PageHeader0x11);

#[repr(packed)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Nom)]
pub struct PageHeaderCommon {
    #[nom(Parse = "{ |i| jet::DateTime::parse_le(i) }")]
    pub database_modification_time: jet::DateTime,
    pub previous_page: uint32_t,
    pub next_page: uint32_t,
    pub father_data_page_object_identifier: uint32_t,
    pub available_data_size: uint16_t,
    pub available_uncommitted_data_size: uint16_t,
    pub available_data_offset: uint16_t,
    pub available_page_tag: uint16_t,
    #[nom(Parse = "{ |i| jet::PageFlags::parse_le(i) }")]
    pub page_flags: PageFlags,
}
impl_read_struct!(PageHeaderCommon);

#[repr(packed)]
#[derive(Copy, Clone, Debug, Nom)]
pub struct PageHeaderExt0x11 {
    pub checksum1: uint64_t,
    pub checksum2: uint64_t,
    pub checksum3: uint64_t,
    pub page_number: uint64_t,
    pub unknown: uint64_t,
}
impl_read_struct!(PageHeaderExt0x11);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum PageHeader {
    old(PageHeaderOld, PageHeaderCommon),
    x0b(PageHeader0x0b, PageHeaderCommon),
    x11(PageHeader0x11, PageHeaderCommon),
    x11_ext(PageHeader0x11, PageHeaderCommon, PageHeaderExt0x11),
}

#[derive(Clone, Debug)]
pub struct PageTag {
    pub size: u16,
    pub offset: u16,
    pub flags: u8,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Nom)]
pub struct RootPageHeader16 {
    pub initial_number_of_pages: uint32_t,
    pub parent_fdp: uint32_t,
    pub extent_space: uint32_t,
    pub space_tree_page_number: uint32_t,
}
impl_read_struct!(RootPageHeader16);

#[repr(packed)]
#[derive(Copy, Clone, Debug, Nom)]
pub struct RootPageHeader25 {
    pub initial_number_of_pages: uint32_t,
    pub unknown1: uint8_t,
    pub parent_fdp: uint32_t,
    pub extent_space: uint32_t,
    pub space_tree_page_number: uint32_t,
    pub unknown2: uint32_t,
    pub unknown3: uint32_t,
}
impl_read_struct!(RootPageHeader25);

#[repr(C)]
#[derive(Debug)]
pub enum RootPageHeader {
    xf(RootPageHeader16),
    x19(RootPageHeader25),
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct BranchPageEntry {
    pub common_page_key_size: uint16_t,
    pub local_page_key_size: uint16_t,
    pub local_page_key: Vec<uint8_t>,
    pub child_page_number: uint32_t,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LeafPageEntry {
    pub common_page_key_size: uint16_t,
    pub local_page_key_size: uint16_t,
    pub local_page_key: Vec<uint8_t>,
    pub child_page_number: uint32_t,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default, Nom)]
pub struct DataDefinitionHeader {
    pub last_fixed_size_data_type: uint8_t,
    pub last_variable_size_data_type: uint8_t,
    pub variable_size_data_types_offset: uint16_t,
}
impl_read_struct!(DataDefinitionHeader);

// Data type identifier: 4 (ColtypOrPgnoFDP)
#[derive(Copy, Clone, Nom)]
#[repr(packed)]
pub struct ColtypOrPgnoFDP {
    fdpn_or_ct: uint32_t,
}

impl ColtypOrPgnoFDP {
    pub fn father_data_page_number(&self) -> uint32_t {
        self.fdpn_or_ct
    }
    pub fn column_type(&self) -> uint32_t {
        self.fdpn_or_ct
    }
}

// Data type identifier: 7 (PagesOrLocale)
#[derive(Copy, Clone, Nom)]
#[repr(packed)]
pub struct PagesOrLocale {
    nop_or_cp_or_li: uint32_t,
}

impl PagesOrLocale {
    // pub fn number_of_pages(&self) -> uint32_t {
    // self.nop_or_cp_or_li
    // }
    pub fn codepage(&self) -> uint32_t {
        self.nop_or_cp_or_li
    }
    // pub fn locale_identifier(&self) -> uint32_t {
    // self.nop_or_cp_or_li
    // }
}

#[repr(packed)]
#[derive(Copy, Clone, Nom)]
pub struct DataDefinition {
    // Data type identifier: 1 (ObjidTable)
    // The father data page (FDP) object identifier
    pub father_data_page_object_identifier: uint32_t,

    // Data type identifier: 2 (Type)
    // The definition type
    pub data_type: uint16_t,

    // Data type identifier: 3 (Id)
    // The indentifier
    pub identifier: uint32_t,

    // Data type identifier: 4 (ColtypOrPgnoFDP)
    #[nom(Parse = "{ |i| ColtypOrPgnoFDP::parse_le(i) }")]
    pub coltyp_or_fdp: ColtypOrPgnoFDP,

    // Data type identifier: 5 (SpaceUsage)
    // The space usage (density percentage)
    pub space_usage: uint32_t,

    // Data type identifier: 6 (Flags)
    // Flags
    pub flags: uint32_t,
    #[nom(Parse = "{ |i| PagesOrLocale::parse_le(i) }")]
    pub pages_or_locale: PagesOrLocale,

    // Data type identifier: 8 (RootFlag)
    // The root flag
    pub root_flag: uint8_t,

    // Data type identifier: 9 (RecordOffset)
    // The record offset
    pub record_offset: uint16_t,

    // Data type identifier: 10 (LCMapFlags)
    // LC Map flags
    pub lc_map_flags: uint32_t,

    // Data type identifier: 11 (KeyMost)
    // Key most
    // Introduced in Windows Vista
    pub key_most: uint16_t,
    /* Data type identifier: 128 (Name)
     * The name
     */

    /* Data type identifier: 129 (Stats)
     */

    /* Data type identifier: 130 (TemplateTable)
     */

    /* Data type identifier: 131 (DefaultValue)
     */

    /* Data type identifier: 132 (KeyFldIDs)
     */

    /* Data type identifier: 133 (VarSegMac)
     */

    /* Data type identifier: 134 (ConditionalColumns)
     */

    /* Data type identifier: 135 (TupleLimits)
     */

    /* Data type identifier: 136 (Version)
     * Introduced in Windows Vista
     */

    /* Data type identifier: 256 (CallbackData)
     */

    /* Data type identifier: 257 (CallbackDependencies)
     */
}
impl_read_struct!(DataDefinition);
