//ese_db
#![allow(non_camel_case_types)]

use crate::parser::jet;
use crate::parser::jet::*;

pub const ESEDB_FILE_SIGNATURE: uint32_t = 0x89abcdef;
pub const ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT: uint32_t = 0x0b;
pub const ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER: uint32_t = 0x11;

#[derive(Copy, Clone, Default, Debug)]
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

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeaderOld {
    pub xor_checksum: uint32_t,
    pub page_number: uint32_t,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeader0x0b {
    pub xor_checksum: uint32_t,
    pub ecc_checksum: uint32_t,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeader0x11 {
    pub checksum: uint64_t,
}

#[repr(packed)]
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

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct PageHeaderExt0x11 {
    pub checksum1: uint64_t,
    pub checksum2: uint64_t,
    pub checksum3: uint64_t,
    pub page_number: uint64_t,
    pub unknown: uint64_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum PageHeader {
    old(PageHeaderOld, PageHeaderCommon),
    x0b(PageHeader0x0b, PageHeaderCommon),
    x11(PageHeader0x11, PageHeaderCommon),
    x11_ext(PageHeader0x11, PageHeaderCommon, PageHeaderExt0x11),
}

#[derive(Debug)]
pub struct PageTag {
    pub size: u16,
    pub offset: u16,
    pub flags: u8,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct RootPageHeader16 {
    pub initial_number_of_pages: uint32_t,
    pub parent_fdp: uint32_t,
    pub extent_space: uint32_t,
    pub space_tree_page_number: uint32_t,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct RootPageHeader25 {
    pub initial_number_of_pages: uint32_t,
    pub unknown1: uint8_t,
    pub parent_fdp: uint32_t,
    pub extent_space: uint32_t,
    pub space_tree_page_number: uint32_t,
    pub unknown2: uint32_t,
    pub unknown3: uint32_t,
}

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
    pub child_page_number: uint32_t
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LeafPageEntry {
    pub common_page_key_size: uint16_t,
    pub local_page_key_size: uint16_t,
    pub local_page_key: Vec<uint8_t>,
    pub child_page_number: uint32_t
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct DataDefinitionHeader {
    pub last_fixed_size_data_type: uint8_t,
    pub last_variable_size_data_type: uint8_t,
    pub variable_size_data_types_offset: uint16_t,
}

// Data type identifier: 4 (ColtypOrPgnoFDP)
#[derive(Copy, Clone)]
#[repr(packed)]
pub union ColtypOrPgnoFDP {
    pub father_data_page_number: uint32_t,
    pub column_type: uint32_t,
}

// Data type identifier: 7 (PagesOrLocale)
#[derive(Copy, Clone)]
#[repr(packed)]
pub union PagesOrLocale {
    // The (initial) number of pages
    pub number_of_pages : uint32_t,

    // The codepage
    pub codepage : uint32_t,

    // The locale identifier
    pub locale_identifier : uint32_t
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct DataDefinition
{
	// Data type identifier: 1 (ObjidTable)
	// The father data page (FDP) object identifier
	pub father_data_page_object_identifier : uint32_t,

	// Data type identifier: 2 (Type)
	// The definition type
	pub data_type : uint16_t,

	// Data type identifier: 3 (Id)
	// The indentifier
	pub identifier : uint32_t,

	// Data type identifier: 4 (ColtypOrPgnoFDP)
    pub coltyp_or_fdp: ColtypOrPgnoFDP,

	// Data type identifier: 5 (SpaceUsage)
	// The space usage (density percentage)
	pub space_usage : uint32_t,

	// Data type identifier: 6 (Flags)
	// Flags
	pub flags : uint32_t,

    pub pages_or_locale: PagesOrLocale,

	// Data type identifier: 8 (RootFlag)
	// The root flag
	pub root_flag : uint8_t,

	// Data type identifier: 9 (RecordOffset)
	// The record offset
	pub record_offset : uint16_t,

	// Data type identifier: 10 (LCMapFlags)
	// LC Map flags
	pub lc_map_flags : uint32_t,

	// Data type identifier: 11 (KeyMost)
	// Key most
	// Introduced in Windows Vista
	pub key_most : uint16_t,

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
