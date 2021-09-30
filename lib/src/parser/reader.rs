//reader.rs
use std::{fs, io, io::{Seek, Read}, mem, path::Path, slice, convert::TryInto, cell::RefCell};
use std::collections::{BTreeSet, HashMap};
use simple_error::SimpleError;
use cache_2q::Cache;

use crate::parser::ese_db;
use crate::parser::ese_db::*;
use crate::parser::jet;
use crate::parser::decomp::*;

#[cfg(target_os = "windows")]
mod gen_db;

mod test;

pub struct Reader {
    file: RefCell<fs::File>,
    cache: RefCell<Cache<u32, Vec<u8>>>,
    format_version: jet::FormatVersion,
    format_revision: jet::FormatRevision,
    page_size: u32,
}

#[allow(clippy::mut_from_ref)]
unsafe fn _any_as_slice<'a, U: Sized, T: Sized>(p: &'a &T) -> &'a [U] {
    slice::from_raw_parts(
        (*p as *const T) as *const U,
        mem::size_of::<T>() / mem::size_of::<U>(),
    )
}

// enum SimpleError {
//     UnsignedError(std::num::TryFromIntError),
// }

// impl TryFrom<std::num::TryFromIntError> for simple_error::SimpleError {
//     fn try_from(error: std::num::TryFromIntError) -> Self {
//         simple_error::SimpleError::UnsignedError(error)
//     }
// }

impl Reader {

    fn load_db_file_header(&mut self) -> Result<ese_db::FileHeader, SimpleError> {

        let mut db_file_header =
            self.read_struct::<ese_db::FileHeader>(0)?;

        if db_file_header.signature != ESEDB_FILE_SIGNATURE {
            return Err(SimpleError::new("bad file_header.signature"));
        }

        fn calc_crc32(file_header: &ese_db::FileHeader) -> u32 {
            let vec32: &[u32] = unsafe { _any_as_slice::<u32, ese_db::FileHeader>(&file_header) };
            vec32.iter().skip(1).fold(0x89abcdef, |crc, &val| crc ^ val)
        }

        let stored_checksum = db_file_header.checksum;
        let checksum = calc_crc32(&db_file_header);
        if stored_checksum != checksum {
            return Err(SimpleError::new(format!("wrong checksum: {}, calculated {}", stored_checksum, checksum)));
        }

        let backup_file_header = self.read_struct::<ese_db::FileHeader>(db_file_header.page_size as u64)?;

        if db_file_header.format_revision == 0 {
            db_file_header.format_revision = backup_file_header.format_revision;
        }

        if db_file_header.format_revision != backup_file_header.format_revision {
            return Err(SimpleError::new(format!(
                "mismatch in format revision: {} not equal to backup value {}",
                db_file_header.format_revision, backup_file_header.format_revision)));
        }

        if db_file_header.page_size == 0 {
            db_file_header.page_size = backup_file_header.page_size;
        }

        if db_file_header.page_size != backup_file_header.page_size {
            return Err(SimpleError::new(format!(
                "mismatch in page size: {} not equal to backup value {}",
                db_file_header.page_size, backup_file_header.page_size)));
        }
        if db_file_header.format_version != 0x620 {
            return Err(SimpleError::new(format!("unsupported format version: {}", db_file_header.format_version)));
        }

        Ok(db_file_header)
    }

    fn new(path: &Path, cache_size: usize) -> Result<Reader, SimpleError> {
        let f = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(SimpleError::new(format!("File::open failed: {:?}", e)))
        };
        let mut reader = Reader {
            file: RefCell::new(f),
            cache: RefCell::new(Cache::new(cache_size)),
            page_size: 2 * 1024, //just to read header
            format_version: 0,
            format_revision: 0,
        };

        let db_fh = reader.load_db_file_header()?;
        reader.format_version = db_fh.format_version;
        reader.format_revision = db_fh.format_revision;
        reader.page_size = db_fh.page_size;

        reader.cache.get_mut().clear();

        Ok(reader)
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), SimpleError> {
        let pg_no = (offset / self.page_size as u64) as u32;
        let mut c = self.cache.borrow_mut();
        if !c.contains_key(&pg_no) {
            let mut page_buf = vec![0u8; self.page_size as usize];
            let mut f = self.file.borrow_mut();
            match f.seek(io::SeekFrom::Start(pg_no as u64 * self.page_size as u64)) {
                Ok(_) => {
                    match f.read_exact(&mut page_buf) {
                        Ok(_) => {
                            c.insert(pg_no, page_buf);
                        },
                        Err(e) => {
                            return Err(SimpleError::new(format!("read_exact failed: {:?}", e)));
                        }
                    }
                },
                Err(e) => {
                    return Err(SimpleError::new(format!("seek failed: {:?}", e)));
                }
            }
        }

        match c.get(&pg_no) {
            Some(page_buf) => {
                let page_offset = (offset % self.page_size as u64) as usize;
                buf.copy_from_slice(&page_buf[page_offset..page_offset + buf.len()]);
            },
            None => {
                return Err(SimpleError::new(format!("Cache failed, page number not found: {}", pg_no)));
            }
        }

        Ok(())
    }

    pub fn read_struct<T>(&self, offset: u64) -> Result<T, SimpleError> {
        let struct_size = mem::size_of::<T>();
        let mut rec: T = unsafe { mem::zeroed() };
        unsafe {
            let buffer = slice::from_raw_parts_mut(&mut rec as *mut _ as *mut u8, struct_size);
            self.read(offset, buffer)?;
        }
        Ok(rec)
    }

    pub fn read_bytes(&self, offset: u64, size: usize) -> Result<Vec<u8>, SimpleError> {
        let mut buf = vec!(0u8; size);
        self.read(offset, &mut buf)?;
        Ok(buf)
    }

    pub fn read_string(&self, offset: u64, size: usize) -> Result<String, SimpleError> {
        let v = self.read_bytes(offset, size)?;
        match std::str::from_utf8(&v) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(SimpleError::new(format!("from_utf8 failed: error_len() is {:?}", e.error_len())))
        }
    }

    pub fn load_db(path: &std::path::Path, cache_size: usize) -> Result<Reader, SimpleError> {
        Reader::new(path, cache_size)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }
}

pub fn load_page_header(
    reader: &Reader,
    page_number: u32,
) -> Result<PageHeader, SimpleError> {
    let page_offset = (page_number + 1) as u64 * (reader.page_size) as u64;

    if reader.format_revision < ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT {
        let header = reader.read_struct::<PageHeaderOld>(page_offset)?;
        let common = reader.read_struct::<PageHeaderCommon>(page_offset + mem::size_of_val(&header) as u64)?;

        //let TODO_checksum = 0;
        Ok(PageHeader::old(header, common))
    } else if reader.format_revision < ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER {
        let header = reader.read_struct::<PageHeader0x0b>(page_offset)?;
        let common = reader.read_struct::<PageHeaderCommon>(page_offset + mem::size_of_val(&header) as u64)?;

        //TODO: verify checksum
        Ok(PageHeader::x0b(header, common))
    } else {
        let header = reader.read_struct::<PageHeader0x11>(page_offset)?;
        let common = reader.read_struct::<PageHeaderCommon>(page_offset + mem::size_of_val(&header) as u64)?;

        //TODO: verify checksum
        if reader.page_size > 8 * 1024 {
            let offs = mem::size_of_val(&header) + mem::size_of_val(&common);
            let ext = reader.read_struct::<PageHeaderExt0x11>(page_offset + offs as u64)?;

            Ok(PageHeader::x11_ext(header, common, ext))
        } else {
            Ok(PageHeader::x11(header, common))
        }
    }
}

pub fn load_page_tags(
    reader: &Reader,
    db_page: &jet::DbPage,
) -> Result<Vec<PageTag>, SimpleError> {
    let page_offset = db_page.offset();
    let mut tags_offset = (page_offset + reader.page_size as u64) as u64;
    let tags_cnt = db_page.get_available_page_tag();
    let mut tags = Vec::<PageTag>::with_capacity(tags_cnt);

    for _i in 0..tags_cnt {
        tags_offset -= 2;
        let page_tag_offset : u16 = reader.read_struct(tags_offset)?;
        tags_offset -= 2;
        let page_tag_size : u16 = reader.read_struct(tags_offset)?;

        let flags : u8;
		let offset : u16;
        let size : u16;

        if reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER && reader.page_size >= 16384 {
			offset = page_tag_offset & 0x7fff;
            size   = page_tag_size & 0x7fff;

            // The upper 3-bits of the first 16-bit-value in the leaf page entry contain the page tag flags
            //if db_page.flags().contains(jet::PageFlags::IS_LEAF)
            {
                let flags_offset = page_offset + db_page.size() as u64 + offset as u64;
                let f : u16 = reader.read_struct(flags_offset)?;
                flags = (f >> 13) as u8;
            }
        } else {
            flags  = (page_tag_offset >> 13) as u8;
            offset = page_tag_offset & 0x1fff;
            size   = page_tag_size & 0x1fff;
        }
        tags.push(PageTag{ flags, offset, size } );
    }

    Ok(tags)
}

pub fn load_root_page_header(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
) -> Result<RootPageHeader, SimpleError> {
    let root_page_offset = page_tag.offset(db_page);

    // TODO Seen in format version 0x620 revision 0x14
    // check format and revision
    if page_tag.size == 16 {
        let root_page_header = reader.read_struct::<ese_db::RootPageHeader16>(root_page_offset)?;
        return Ok(RootPageHeader::xf(root_page_header));
    } else if page_tag.size == 25 {
        let root_page_header = reader.read_struct::<ese_db::RootPageHeader25>(root_page_offset)?;
        return Ok(RootPageHeader::x19(root_page_header));
    }

    Err(SimpleError::new(format!("wrong size of page tag: {:?}", page_tag)))
}

pub fn page_tag_get_branch_child_page_number(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
) -> Result<u32, SimpleError> {
    let mut offset = page_tag.offset(db_page);

    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        offset += 2;
    }
    let local_page_key_size : u16 = reader.read_struct(offset)?;
    offset += 2;
    offset += local_page_key_size as u64;

    let child_page_number : u32 = reader.read_struct(offset)?;
    Ok(child_page_number)
}

pub fn load_catalog(
    reader: &Reader,
) -> Result<Vec<jet::TableDefinition>, SimpleError> {
    let db_page = jet::DbPage::new(reader, jet::FixedPageNumber::Catalog as u32)?;
    let pg_tags = &db_page.page_tags;

    let is_root = db_page.flags().contains(jet::PageFlags::IS_ROOT);

    if is_root {
        let _root_page_header = load_root_page_header(reader, &db_page, &pg_tags[0])?;
    }

    let mut res : Vec<jet::TableDefinition> = vec![];
    let mut table_def : jet::TableDefinition = jet::TableDefinition { table_catalog_definition: None,
        column_catalog_definition_array: vec![], long_value_catalog_definition: None };

    let mut page_number;
    if db_page.flags().contains(jet::PageFlags::IS_PARENT) {
        page_number = page_tag_get_branch_child_page_number(reader, &db_page, &pg_tags[1])?;
    } else if db_page.flags().contains(jet::PageFlags::IS_LEAF) {
            page_number = db_page.page_number;
        } else {
            return Err(SimpleError::new(format!("pageno {}: neither IS_PARENT nor IS_LEAF is present in {:?}",
                                                db_page.page_number, db_page.flags())));
    }
    let mut prev_page_number = db_page.page_number;

    while page_number != 0 {
        let db_page = jet::DbPage::new(reader, page_number)?;
        let pg_tags = &db_page.page_tags;

        if db_page.prev_page() != 0 && prev_page_number != db_page.prev_page() {
            return Err(SimpleError::new(format!("pageno {}: wrong previous_page number {}, expected {}",
                db_page.page_number, db_page.prev_page(), prev_page_number)));
        }
        if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
            return Err(SimpleError::new(format!("pageno {}: IS_LEAF flag should be present",
                db_page.page_number)));
        }

        for i in pg_tags.iter().skip(1) {
            if jet::PageTagFlags::from_bits_truncate(i.flags).intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT) {
                continue;
            }
            let cat_item = load_catalog_item(reader, &db_page, i)?;
            if cat_item.cat_type == jet::CatalogType::Table as u16 {
                if table_def.table_catalog_definition.is_some() {
                    res.push(table_def);
                    table_def = jet::TableDefinition { table_catalog_definition: None,
                        column_catalog_definition_array: vec![], long_value_catalog_definition: None };
                } else if !table_def.column_catalog_definition_array.is_empty() ||
                    table_def.long_value_catalog_definition.is_some() {
                    return Err(SimpleError::new(
                        "corrupted table detected: column/long definition is going before table"));
                }
                table_def.table_catalog_definition = Some(cat_item);
            } else if cat_item.cat_type == jet::CatalogType::Column as u16 {
                table_def.column_catalog_definition_array.push(cat_item);
            } else if cat_item.cat_type == jet::CatalogType::Index as u16 {
                // TODO
            } else if cat_item.cat_type == jet::CatalogType::LongValue as u16 {
                if table_def.long_value_catalog_definition.is_some() {
                    return Err(SimpleError::new("long-value catalog definition duplicate?"));
                }
                table_def.long_value_catalog_definition = Some(cat_item);
            } else if cat_item.cat_type == jet::CatalogType::Callback as u16 {
                // TODO
            } else {
                println!("TODO: Unknown cat_item.cat_type {}", cat_item.cat_type);
            }
        }
        prev_page_number = page_number;
        page_number = db_page.next_page();
    }

    if table_def.table_catalog_definition.is_some() {
        res.push(table_def);
    }

    Ok(res)
}

pub fn load_catalog_item(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
) -> Result<jet::CatalogDefinition, SimpleError> {
    let mut offset = page_tag.offset(db_page);

    let mut first_word_read = false;
    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        first_word_read = true;
        offset += 2;
    }
    let mut local_page_key_size : u16 = reader.read_struct(offset)?;
    if !first_word_read {
        local_page_key_size = clean_pgtag_flag(reader, db_page, local_page_key_size);
    }
    offset += 2;
    offset += local_page_key_size as u64;

    let offset_ddh = offset;
    let ddh = reader.read_struct::<ese_db::DataDefinitionHeader>(offset_ddh)?;
    offset += mem::size_of::<ese_db::DataDefinitionHeader>() as u64;

    let number_of_variable_size_data_types : u32;
    if ddh.last_variable_size_data_type > 127 {
        number_of_variable_size_data_types = ddh.last_variable_size_data_type as u32 - 127;
    } else {
        number_of_variable_size_data_types = 0;
    }

    let cat_def_zero = std::mem::MaybeUninit::<jet::CatalogDefinition>::zeroed();
    let mut cat_def = unsafe { cat_def_zero.assume_init() };
    let data_def = reader.read_struct::<ese_db::DataDefinition>(offset)?;

    cat_def.father_data_page_object_identifier = data_def.father_data_page_object_identifier;
    cat_def.cat_type = data_def.data_type;
    cat_def.identifier = data_def.identifier;
    if cat_def.cat_type == jet::CatalogType::Column as u16 {
        cat_def.column_type = unsafe { data_def.coltyp_or_fdp.column_type };
    } else {
        cat_def.father_data_page_number = unsafe { data_def.coltyp_or_fdp.father_data_page_number };
    }
    cat_def.size = data_def.space_usage;
    cat_def.flags = data_def.flags;
    if cat_def.cat_type == jet::CatalogType::Column as u16 {
        cat_def.codepage = unsafe { data_def.pages_or_locale.codepage };
    }
    if ddh.last_fixed_size_data_type >= 10 {
        cat_def.lcmap_flags = data_def.lc_map_flags;
    }

    if number_of_variable_size_data_types > 0 {
        let mut variable_size_data_types_offset = ddh.variable_size_data_types_offset as u32;
        let variable_size_data_type_value_data_offset = variable_size_data_types_offset + (number_of_variable_size_data_types * 2);
        let mut previous_variable_size_data_type_size : u16 = 0;
        let mut data_type_number : u16 = 128;
        for _ in 0..number_of_variable_size_data_types {
            offset += ddh.variable_size_data_types_offset as u64;
            let variable_size_data_type_size : u16 = reader.read_struct(offset_ddh + variable_size_data_types_offset as u64)?;
            variable_size_data_types_offset += 2;

            let data_type_size : u16;
            if variable_size_data_type_size & 0x8000 != 0 {
                data_type_size = 0;
            } else {
                data_type_size = variable_size_data_type_size - previous_variable_size_data_type_size;
            }
            if data_type_size > 0 {
                match data_type_number {
                    128 => {
                        let offset_dtn = offset_ddh + variable_size_data_type_value_data_offset as u64 + previous_variable_size_data_type_size as u64;
                        cat_def.name = reader.read_string(offset_dtn, data_type_size as usize)?;
                    },
                    130 => {
                        // TODO template_name
                    },
                    131 => {
                        // TODO default_value
                        let offset_def = offset_ddh + variable_size_data_type_value_data_offset as u64 + previous_variable_size_data_type_size as u64;
                        cat_def.default_value = reader.read_bytes(offset_def, data_type_size as usize)?;
                    },
                    132 | // KeyFldIDs
                    133 | // VarSegMac
                    134 | // ConditionalColumns
                    135 | // TupleLimits
                    136   // Version
                        => {
                        // not useful fields
                    },
                    _ => {
                        if data_type_size > 0 {
                            println!("TODO handle data_type_number {}", data_type_number);
                        }
                    }
                }
                previous_variable_size_data_type_size = variable_size_data_type_size;
			}
			data_type_number += 1;
        }
    }

    Ok(cat_def)
}

pub fn clean_pgtag_flag(reader: &Reader, db_page: &jet::DbPage, data: u16) -> u16 {
    // The upper 3-bits of the first 16-bit-value in the leaf page entry contain the page tag flags
    if reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER
        && reader.page_size >= 16384
        && db_page.flags().contains(jet::PageFlags::IS_LEAF)
    {
        return data & 0x1FFF;
    }
    data
}

pub fn find_first_leaf_page(reader: &Reader, mut page_number: u32)
    -> Result<u32, SimpleError> {
    let mut visited_pages : BTreeSet<u32> = BTreeSet::new();
    loop {
        if visited_pages.contains(&page_number) {
            return Err(SimpleError::new(format!("Child page loop detected at page number {}, visited pages: {:?}",
                page_number, visited_pages)));
        }

        let db_page = jet::DbPage::new(reader, page_number)?;
        if db_page.flags().contains(jet::PageFlags::IS_LEAF) {
            return Ok(page_number);
        } else {
            visited_pages.insert(page_number);
        }

        let pg_tags = &db_page.page_tags;
        page_number = page_tag_get_branch_child_page_number(reader, &db_page, &pg_tags[1])?;
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TaggedDataState {
    pub identifier: u16,
    pub types_offset: u16,
    pub type_offset: u16,
    pub offset_data_size: u16,
    pub remaining_definition_data_size: u16,
	pub tagged_data_type_size: u16,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct VariableSizeDataState {
    pub current_type: u32,
    pub type_offset: u16,
    pub value_offset: u16,
}

#[derive(Clone, Debug, Default)]
pub struct LastLoadState {
	pub page_number: u32,
    pub page_tag_index: usize,
	pub last_column: u32,
	pub offset: u64,
	pub offset_ddh: u64,
	pub record_data_size: u64,
	pub ddh: ese_db::DataDefinitionHeader,
	pub fixed_data_bits_mask_size: usize,
	pub fixed_data_bits_mask : Vec<u8>,
	pub tag_state: TaggedDataState,
	pub previous_variable_size_data_type_size: u16,
	pub var_state: VariableSizeDataState,
}

impl LastLoadState {
	pub fn init(page_number: u32, page_tag_index: usize) -> Self {
        LastLoadState { page_number, page_tag_index, ..Default::default() }
	}
}

#[allow(clippy::too_many_arguments)]
pub fn load_data(
	lls: &mut LastLoadState,
    reader: &Reader,
    tbl_def: &jet::TableDefinition,
    lv_tags: &LV_tags,
    db_page: &jet::DbPage,
    page_tag_index: usize,
    column_id: u32,
    multi_value_index: usize // 0 value mean itagSequence = 1
) -> Result<Option<Vec<u8>>, SimpleError> {

    let pg_tags = &db_page.page_tags;

    if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
        return Err(SimpleError::new(format!("expected leaf page, page_flags 0x{:?}",
            db_page.flags())));
    }

    if page_tag_index == 0 || page_tag_index >= pg_tags.len() {
        return Err(SimpleError::new(format!("wrong page tag index: {}", page_tag_index)));
    }

    let page_tag = &pg_tags[page_tag_index];

	let mut tagged_data_types_format = jet::TaggedDataTypesFormats::Index;
	if reader.format_version == 0x620 && reader.format_revision <= 2 {
		tagged_data_types_format = jet::TaggedDataTypesFormats::Linear;
	}

	let mut start_i = 0;
	if lls.last_column == 0 {
		lls.offset = page_tag.offset(db_page);
		let offset_start = lls.offset;

		let mut first_word_read = false;
		if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
			first_word_read = true;
			lls.offset += 2;
		}
		let mut local_page_key_size : u16 = reader.read_struct(lls.offset)?;
		if !first_word_read {
			local_page_key_size = clean_pgtag_flag(reader, db_page, local_page_key_size);
		}
		lls.offset += 2;
		lls.offset += local_page_key_size as u64;

		lls.record_data_size = page_tag.size as u64 - (lls.offset - offset_start);

		lls.offset_ddh = lls.offset;
		lls.ddh = reader.read_struct::<ese_db::DataDefinitionHeader>(lls.offset_ddh)?;
		lls.offset += mem::size_of::<ese_db::DataDefinitionHeader>() as u64;

		// read fixed data bits mask, located at the end of fixed columns
		lls.fixed_data_bits_mask_size = (lls.ddh.last_fixed_size_data_type as usize + 7) / 8;
		if lls.fixed_data_bits_mask_size > 0 {
			lls.fixed_data_bits_mask = reader.read_bytes(
				lls.offset_ddh + lls.ddh.variable_size_data_types_offset as u64 - lls.fixed_data_bits_mask_size as u64,
				lls.fixed_data_bits_mask_size)?;
		}

		let number_of_variable_size_data_types : u16;
		if lls.ddh.last_variable_size_data_type > 127 {
			number_of_variable_size_data_types = lls.ddh.last_variable_size_data_type as u16 - 127;
		} else {
			number_of_variable_size_data_types = 0;
		}

		lls.var_state.current_type = 127;
		lls.var_state.type_offset = lls.ddh.variable_size_data_types_offset;
		lls.var_state.value_offset = lls.ddh.variable_size_data_types_offset + (number_of_variable_size_data_types * 2);
	} else {
		for j in 0..tbl_def.column_catalog_definition_array.len() {
			let col = &tbl_def.column_catalog_definition_array[j];
			if col.identifier == lls.last_column {
				start_i = j;
				break;
			}
		}
	}

    for i in start_i..tbl_def.column_catalog_definition_array.len() {
        let col = &tbl_def.column_catalog_definition_array[i];
        if col.identifier <= 127 {
            if col.identifier <= lls.ddh.last_fixed_size_data_type as u32 {
                // fixed size column
                if col.identifier == column_id {
                    if lls.fixed_data_bits_mask_size > 0 && lls.fixed_data_bits_mask[i/8] & (1 << (i % 8)) > 0 {
                        // empty value
                        return Ok(None);
                    }
                    let v = reader.read_bytes(lls.offset, col.size as usize)?;
                    return Ok(Some(v));
                }
                lls.offset += col.size as u64;
            } else if col.identifier == column_id {
                // no value in tag
                return Ok(None);
            }
        } else if lls.var_state.current_type < lls.ddh.last_variable_size_data_type as u32 {
            // variable size
            while lls.var_state.current_type < col.identifier {
                let variable_size_data_type_size : u16 = reader.read_struct(lls.offset_ddh + lls.var_state.type_offset as u64)?;
                lls.var_state.type_offset += 2;
                lls.var_state.current_type += 1;
                if lls.var_state.current_type == col.identifier && (variable_size_data_type_size & 0x8000) == 0 {                    
                    let var_offset = lls.offset_ddh + lls.var_state.value_offset as u64;
                    let var_size = variable_size_data_type_size - lls.previous_variable_size_data_type_size;

                    lls.var_state.value_offset += var_size;
                    lls.previous_variable_size_data_type_size = variable_size_data_type_size;

                    if col.identifier == column_id {
                        let v = reader.read_bytes(var_offset, var_size as usize)?;
                        return Ok(Some(v));
                    }
                }
                if lls.var_state.current_type >= lls.ddh.last_variable_size_data_type as u32 {
                    break;
                }
            }
        } else {
            // tagged
            if tagged_data_types_format == jet::TaggedDataTypesFormats::Linear {
                // TODO
                println!("TODO tagged_data_types_format ==-- jet::TaggedDataTypesFormats::Linear");
            } else if tagged_data_types_format == jet::TaggedDataTypesFormats::Index {
                match load_tagged_data_linear(reader, lv_tags, col, column_id, &mut lls.tag_state, &mut lls.var_state,
                    &mut lls.offset, lls.offset_ddh, lls.record_data_size, multi_value_index) {
                    Err(e) => return Err(e),
                    Ok(r) => {
                        if r.is_some() {
                            return Ok(r);
                        }
                    }
                }
            }
        }
        // column not found?
        if col.identifier == column_id {
            // default present?
            if !col.default_value.is_empty() {
                return Ok(Some(col.default_value.clone()));
            }
            // empty
            return Ok(None)
        }
    }

    Err(SimpleError::new(format!("column {} not found", column_id)))
}

fn init_tag_state(
    tag_state: &mut TaggedDataState,
    reader: &Reader,
    var_state: VariableSizeDataState,
    offset: &mut u64,
    offset_ddh : u64,
    record_data_size: u64,
) -> Result<Option<Vec<u8>>, SimpleError> {
    tag_state.types_offset = var_state.value_offset;
    tag_state.remaining_definition_data_size =
        (record_data_size - tag_state.types_offset as u64)
        .try_into()
        .map_err(|e: std::num::TryFromIntError| SimpleError::new(e.to_string()))?;
        

    *offset = offset_ddh + tag_state.types_offset as u64;

    if tag_state.remaining_definition_data_size > 0 {
        tag_state.identifier = reader.read_struct::<u16>(*offset)?;
        *offset += 2;

        tag_state.type_offset = reader.read_struct::<u16>(*offset)?;
        *offset += 2;

        if tag_state.type_offset == 0 {
            return Err(SimpleError::new("tag_state.type_offset == 0"));
        }
        tag_state.offset_data_size = (tag_state.type_offset & 0x3fff) - 4;
        tag_state.remaining_definition_data_size -= 4;
    }
    Ok(None)
}
#[allow(clippy::too_many_arguments)]
fn load_tagged_data_linear(
    reader: &Reader,
    lv_tags: &LV_tags,
    col: &jet::CatalogDefinition,
    column_id: u32,
    tag_state: &mut TaggedDataState,
    var_state: &mut VariableSizeDataState,
    offset : &mut u64,
    offset_ddh : u64,
    record_data_size: u64,
    multi_value_index: usize,
) -> Result<Option<Vec<u8>>, SimpleError> {
    if tag_state.types_offset == 0 {
        init_tag_state(tag_state, reader, *var_state, offset, offset_ddh, record_data_size)?;
    }
    if tag_state.remaining_definition_data_size > 0 && col.identifier == tag_state.identifier as u32 {
        let previous_tagged_data_type_offset = tag_state.type_offset;
        if tag_state.offset_data_size > 0 {
            tag_state.identifier = reader.read_struct::<u16>(*offset)?;
            *offset += 2;

            tag_state.type_offset = reader.read_struct::<u16>(*offset)?;
            *offset += 2;

            tag_state.offset_data_size               -= 4;
            tag_state.remaining_definition_data_size -= 4;
        }

        let tagged_data_type_offset_bitmask : u16;
        if reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER && reader.page_size >= 16384 {
            tagged_data_type_offset_bitmask = 0x7fff;
        } else {
            tagged_data_type_offset_bitmask = 0x3fff;
        }
        let masked_previous_tagged_data_type_offset : u16 =
            previous_tagged_data_type_offset & tagged_data_type_offset_bitmask;
        let masked_tagged_data_type_offset = tag_state.type_offset & tagged_data_type_offset_bitmask;

        if masked_tagged_data_type_offset > masked_previous_tagged_data_type_offset {
            tag_state.tagged_data_type_size = masked_tagged_data_type_offset - masked_previous_tagged_data_type_offset;
        } else {
            tag_state.tagged_data_type_size = tag_state.remaining_definition_data_size;
        }
        let mut tagged_data_type_value_offset = tag_state.types_offset + masked_previous_tagged_data_type_offset;
        let mut data_type_flags : u8 = 0;
        if tag_state.tagged_data_type_size > 0 {
            tag_state.remaining_definition_data_size -= tag_state.tagged_data_type_size;
            if (reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER &&
                reader.page_size >= 16384) || (previous_tagged_data_type_offset & 0x4000 ) != 0
            {
                data_type_flags = reader.read_struct(offset_ddh + tagged_data_type_value_offset as u64)?;

                tagged_data_type_value_offset	+= 1;
                tag_state.tagged_data_type_size -= 1;
            }
        }
        if tag_state.tagged_data_type_size > 0 && col.identifier == column_id {
            let value_offset = offset_ddh + tagged_data_type_value_offset as u64;
            match load_tagged_column(reader, lv_tags, col, value_offset, tag_state.tagged_data_type_size, data_type_flags,
                multi_value_index) {
                Err(e) => return Err(e),
                Ok(r) => {
                    if r.is_some() {
                        return Ok(r);
                    }
                }
            }
        }
    }
    Ok(None)
}

fn load_tagged_column(
    reader: &Reader,
    lv_tags: &LV_tags,
    col: &jet::CatalogDefinition,
    offset : u64,
    tagged_data_type_size: u16,
    data_type_flags: u8,
    multi_value_index: usize,
) -> Result<Option<Vec<u8>>, SimpleError> {
    let mut v = Vec::new();

    use jet::ColumnFlags;
    use jet::TaggedDataTypeFlag;

    let col_flag = ColumnFlags::from_bits_truncate(col.flags);
    let compressed = col_flag.intersects(ColumnFlags::Compressed);
    let dtf = TaggedDataTypeFlag::from_bits_truncate(data_type_flags as u16);
    if dtf.intersects(TaggedDataTypeFlag::LONG_VALUE) {
        let key = reader.read_struct::<u32>(offset)?;
        v = load_lv_data(reader, lv_tags, key, compressed)?;
    } else if dtf.intersects(TaggedDataTypeFlag::MULTI_VALUE | TaggedDataTypeFlag::MULTI_VALUE_OFFSET) {
        let mv = read_multi_value(
            reader, offset, tagged_data_type_size, &dtf, multi_value_index, lv_tags, compressed)?;
        if let Some(mv_data) = mv {
            v = mv_data;
        }
    } else if dtf.intersects(jet::TaggedDataTypeFlag::COMPRESSED) {
        v = reader.read_bytes(offset, tagged_data_type_size as usize)?;
        let dsize = decompress_size(&v);
        if dsize > 0 {
            v = decompress_buf(&v, dsize)?;
        }
    } else {
        v = reader.read_bytes(offset, tagged_data_type_size as usize)?;
    }

    if !v.is_empty() {
        return Ok(Some(v));
    }
    Ok(None)
}

fn read_multi_value(
    reader: &Reader,
    offset: u64,
    tagged_data_type_size: u16,
    dtf: &jet::TaggedDataTypeFlag,
    multi_value_index: usize,
    lv_tags: &LV_tags,
    compressed: bool
) -> Result<Option<Vec<u8>>, SimpleError> {
    let mut mv_indexes : Vec<(u16/*shift*/, (bool/*lv*/, u16/*size*/))> = Vec::new();
    if dtf.intersects(jet::TaggedDataTypeFlag::MULTI_VALUE_OFFSET) {
        // The first byte contain the offset
        // [13, ...]
        let offset_mv_list = offset;
        let value : u16 = reader.read_struct::<u8>(offset_mv_list)? as u16;

        mv_indexes.push((1, (false, value)));
        mv_indexes.push((value+1, (false, tagged_data_type_size - value - 1)));
    } else if dtf.intersects(jet::TaggedDataTypeFlag::MULTI_VALUE) {
        // The first 2 bytes contain the offset to the first value
        // there is an offset for every value
        // therefore first offset / 2 = the number of value entries
        // [8, 0, 7, 130, 11, 2, 10, 131, ...]
        let mut offset_mv_list = offset;
        let mut value = reader.read_struct::<u16>(offset_mv_list)?;
        offset_mv_list += 2;

        let mut value_entry_size : u16;
        let mut value_entry_offset = value & 0x7fff;
        let mut entry_lvbit : bool = (value & 0x8000) > 0;
        let number_of_value_entries = value_entry_offset / 2;

        for _ in 1..number_of_value_entries {
            value = reader.read_struct::<u16>(offset_mv_list)?;
            offset_mv_list += 2;
            value_entry_size = (value & 0x7fff) - value_entry_offset;
            mv_indexes.push((value_entry_offset, (entry_lvbit, value_entry_size)));
            entry_lvbit = (value & 0x8000) > 0;
            value_entry_offset = value & 0x7fff;
        }
        value_entry_size = tagged_data_type_size - value_entry_offset;
        mv_indexes.push((value_entry_offset, (entry_lvbit, value_entry_size)));
    } else {
        return Err(SimpleError::new(format!("Unknown TaggedDataTypeFlag: {}", dtf.bits())));
    }
    let mut mv_index = 0;
    if multi_value_index > 0 && multi_value_index - 1 < mv_indexes.len() {
        mv_index = multi_value_index - 1;
    }

    if mv_index < mv_indexes.len() {
        let (shift, (lv, size)) = mv_indexes[mv_index];
        let v;
        if lv {
            let key = reader.read_struct::<u32>(offset + shift as u64)?;
            v = load_lv_data(reader, lv_tags, key, compressed)?;
        } else {
            v = reader.read_bytes(offset + shift as u64, size as usize)?;
            if compressed {
                let dsize = decompress_size(&v);
                if dsize > 0 {
                    let dv = decompress_buf(&v, dsize)?;
                    return Ok(Some(dv));
                }
            }
        }
        return Ok(Some(v));
    }
    Ok(None)
}

pub fn load_lv_tag(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
    page_tag_0: &PageTag
) -> Result<Option<LV_tags>, SimpleError> {
    let mut offset = page_tag.offset(db_page);
    let page_tag_offset : u64 = offset;

    let mut res = LV_tag { common_page_key: vec![], local_page_key: vec![], offset: 0, size: 0 };

    let mut first_word_read = false;
    let mut common_page_key_size : u16 = 0;
    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        common_page_key_size = clean_pgtag_flag(reader, db_page, reader.read_struct::<u16>(offset)?);
        first_word_read = true;
        offset += 2;

        if common_page_key_size > 0 {
            let offset0 = page_tag_0.offset(db_page);
            let mut common_page_key = reader.read_bytes(offset0, common_page_key_size as usize)?;
            res.common_page_key.append(&mut common_page_key);
        }
    }

    let mut local_page_key_size : u16 = reader.read_struct(offset)?;
    if !first_word_read {
        local_page_key_size = clean_pgtag_flag(reader, db_page, local_page_key_size);
    }
    offset += 2;
    if local_page_key_size > 0 {
        let mut local_page_key = reader.read_bytes(offset, local_page_key_size as usize)?;
        res.local_page_key.append(&mut local_page_key);
        offset += local_page_key_size as u64;
    }

    if (page_tag.size as u64) - (offset - page_tag_offset) == 8 {
        //let _skey: u32 = reader.read_struct(offset)?;
        //offset += 4;
        //let _total_size : u32 = reader.read_struct(offset)?;
		// TODO: handle? page_tags with skey & total_size only (seems don't need)
        Ok(None)
    } else {
        let mut page_key: Vec<u8> = vec![];
        if common_page_key_size + local_page_key_size == 8 {
            page_key.append(&mut res.common_page_key.clone());
            page_key.append(&mut res.local_page_key.clone());
        } else if local_page_key_size >= 4 {
            page_key = res.local_page_key.clone();
        } else if common_page_key_size >= 4 {
            page_key = res.common_page_key.clone();
        }

        let skey = unsafe {
            match page_key[0..4].try_into() {
                Ok(pk) => std::mem::transmute::<[u8; 4], u32>(pk),
                Err(e) => return Err(SimpleError::new(format!("can't convert page_key {:?} into slice [0..4], error: {}",
                    page_key, e)))
            }
        }.to_be();

		let mut seg_offset = 0;

        if page_key.len() == 8 {
            // let segment_offset = unsafe {
            //     std::mem::transmute::<[u8; 4], u32>(page_key[4..8].try_into().unwrap())
            // }.to_be();
            let segment_offset = u32::from_le_bytes(page_key[4..8].try_into().expect("Bad slice length")).to_be();
            seg_offset = segment_offset;
        }

        res.offset = offset;
        res.size = (page_tag.size as u64 - (offset - page_tag_offset)).try_into().expect("Bad res size");

		let mut t : HashMap<u32, LV_tag> = HashMap::new();
		t.insert(seg_offset, res);
		let mut new_tag : LV_tags = HashMap::new();
		new_tag.insert(skey, t);

        Ok(Some(new_tag))
    }
}

#[derive(Debug, Clone)]
pub struct LV_tag {
    pub common_page_key: Vec<u8>,
    pub local_page_key: Vec<u8>,
    pub offset: u64,
    pub size: u32,
}

pub type LV_tags = HashMap<u32/*key*/, HashMap<u32/*seg_offset*/, LV_tag>>;

fn merge_lv_tags(tags: &mut LV_tags, new_tags: LV_tags) {
	for (new_key, new_segs) in new_tags {
        if let std::collections::hash_map::Entry::Vacant(e) = tags.entry(new_key) {
            let r : std::option::Option<u32> = { e.insert(new_segs); None };
            debug_assert!(r.is_none(), "new_key wasn't there before insert fn called!");
        } else {
            let segs = tags.get_mut(&new_key).expect("No new_key found");
			for (new_seg_offset, new_lv_tags) in new_segs {
				let r = segs.insert(new_seg_offset, new_lv_tags);
				assert!(r.is_none(), "{}", true);
			}
        }
        
	}
}

pub fn load_lv_metadata(
    reader: &Reader,
    page_number: u32
) -> Result<LV_tags, SimpleError> {
    let db_page = jet::DbPage::new(reader, page_number)?;
    let pg_tags = &db_page.page_tags;

    if !db_page.flags().contains(jet::PageFlags::IS_LONG_VALUE) {
        return Err(SimpleError::new(format!("pageno {}: IS_LONG_VALUE flag should be present",
            db_page.page_number)));
    }

    let mut tags : LV_tags = HashMap::new();

    if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
        let mut prev_page_number = page_number;
        let mut page_number = page_tag_get_branch_child_page_number(reader, &db_page, &pg_tags[1])?;
        while page_number != 0 {
            let db_page = jet::DbPage::new(reader, page_number)?;
            let pg_tags = &db_page.page_tags;

            if db_page.prev_page() != 0 && prev_page_number != db_page.prev_page() {
                return Err(SimpleError::new(format!("pageno {}: wrong previous_page number {}, expected {}",
                    db_page.page_number, db_page.prev_page(), prev_page_number)));
            }
            if !db_page.flags().contains(jet::PageFlags::IS_LEAF | jet::PageFlags::IS_LONG_VALUE) {
                // maybe it's "Parent of leaf" page
                let r = load_lv_metadata(reader, page_number);
                match r {
                    Ok(new_tags) => {
						merge_lv_tags(&mut tags, new_tags);
                    },
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                for i in 1..pg_tags.len() {
                    if jet::PageTagFlags::from_bits_truncate(pg_tags[i].flags).intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT) {
                        continue;
                    }

					match load_lv_tag(reader, &db_page, &pg_tags[i], &pg_tags[0]) {
						Ok(r) => {
							if let Some(new_tag) = r {
								merge_lv_tags(&mut tags, new_tag);
							}
						},
						Err(e) => return Err(e)
					}
                }
            }
            prev_page_number = page_number;
            page_number = db_page.next_page();
        }
    } else {
        for i in 1..pg_tags.len() {
            match load_lv_tag(reader, &db_page, &pg_tags[i], &pg_tags[0]) {
                Ok(r) => {
					if let Some(new_tag) = r {
						merge_lv_tags(&mut tags, new_tag);
					}
                },
                Err(e) => return Err(e)
            }
        }
    }

    Ok(tags)
}

pub fn load_lv_data(
    reader: &Reader,
    lv_tags: &LV_tags,
    long_value_key: u32,
    compressed: bool
) -> Result<Vec<u8>, SimpleError> {
    let mut res : Vec<u8> = vec![];
	if lv_tags.contains_key(&long_value_key) {
		let seg_offsets = lv_tags.get(&long_value_key).expect("No long value key");
		loop {
			let offset = res.len() as u32;
			if seg_offsets.contains_key(&offset) {
				let tag = seg_offsets.get(&offset).expect("No offset");
				let mut v = reader.read_bytes(tag.offset, tag.size as usize)?;
				if compressed {
					let dsize = decompress_size(&v);
					if dsize > 0 {
						v = decompress_buf(&v, dsize)?;
					}
				}
				res.append(&mut v);
				// search next offset
			} else {
				break;
			}
		}
    }

    if !res.is_empty() {
        Ok(res)
    } else {
        Err(SimpleError::new(format!("LV key {} not found", long_value_key)))
    }
}
