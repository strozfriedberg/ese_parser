//reader.rs

use std::{fs, io, io::{Seek, Read}, mem, path::PathBuf, slice, convert::TryInto, cell::RefCell};
use simple_error::SimpleError;
use cache_2q::Cache;

use crate::parser::ese_db;
use crate::parser::ese_db::*;
use crate::parser::jet;

pub struct Reader {
    path: PathBuf,
    file: RefCell<fs::File>,
    cache: RefCell<Cache<u32, Vec<u8>>>,
    format_version: jet::FormatVersion,
    format_revision: jet::FormatRevision,
    page_size: u32,
    last_page_number: u32,
}

impl Reader {
    fn read(&self, offset: u64, buf: &mut [u8]) {
        let pg_no = (offset / self.page_size as u64) as u32;

        if !self.cache.borrow_mut().contains_key(&pg_no) {
            let mut page_buf = vec![0u8; self.page_size as usize];

            self.file.borrow_mut().seek(io::SeekFrom::Start(pg_no as u64 * self.page_size as u64)).unwrap();
            if let Ok(_) = self.file.borrow_mut().read_exact(&mut page_buf) {
                self.cache.borrow_mut().insert(pg_no, page_buf);
            }
        }

        if let Some(page_buf) = self.cache.borrow_mut().get(&pg_no) {
            let page_offset = (offset % self.page_size as u64) as usize;
            buf.copy_from_slice(&page_buf[page_offset..page_offset + buf.len()]);
        }
    }

    pub fn read_struct<T>(&self, offset: u64) -> T {
        let struct_size = mem::size_of::<T>();
        let mut rec: T = unsafe { mem::zeroed() };
        unsafe {
            let buffer = slice::from_raw_parts_mut(&mut rec as *mut _ as *mut u8, struct_size);
            self.read(offset, buffer);
        }
        rec
    }

    fn new(path: &PathBuf, cache_size: usize) -> Reader {
        let mut reader = Reader {
            path: path.clone(),
            file: RefCell::new(fs::File::open(path).unwrap()),
            cache: RefCell::new(Cache::new(cache_size)),
            page_size: 2 * 1024, //just to read header
            format_version: 0,
            format_revision: 0,
            last_page_number: 0
        };
        let db_fh = reader.read_struct::<ese_db::FileHeader>(0);

        reader.format_version = db_fh.format_version;
        reader.format_revision = db_fh.format_revision;
        reader.page_size = db_fh.page_size;
        reader.last_page_number = (db_fh.page_size * 2) / db_fh.page_size;

        reader.cache.get_mut().remove(&0);

        reader
    }

    pub fn read_bytes(&self, offset: u64, size: usize) -> Vec<u8> {
        let mut buf = vec!(0u8; size);
        self.read(offset, &mut buf);
        buf
    }

    pub fn read_string(&mut self, offset: u64, size: usize) -> String {
        let v = self.read_bytes(offset, size);
        std::str::from_utf8(&v).unwrap().to_string()
    }

    pub fn load_db(path: &std::path::PathBuf, cache_size: usize) -> Result<Reader, SimpleError> {
        Ok(Reader::new(path, cache_size))
    }
}

/// # Safety
///
/// use slice::from_raw_parts_mut
#[allow(clippy::mut_from_ref)]
unsafe fn _any_as_slice<'a, U: Sized, T: Sized>(p: &'a &mut T) -> &'a mut [U] {
    slice::from_raw_parts_mut(
        (*p as *const T) as *mut U,
        mem::size_of::<T>() / mem::size_of::<U>(),
    )
}

pub fn load_db_file_header(reader: &mut Reader) -> Result<ese_db::FileHeader, SimpleError> {
    let mut db_file_header =
        reader.read_struct::<ese_db::FileHeader>(0);

    assert_eq!(
        db_file_header.signature, ESEDB_FILE_SIGNATURE,
        "bad file_header.signature"
    );

    fn calc_crc32(file_header: &&mut ese_db::FileHeader) -> u32 {
        let vec32: &[u32] = unsafe { _any_as_slice::<u32, _>(&file_header) };
        vec32.iter().skip(1).fold(0x89abcdef, |crc, &val| crc ^ val)
    }

    let stored_checksum = db_file_header.checksum;
    let checksum = calc_crc32(&&mut db_file_header);
    if stored_checksum != checksum {
        return Err(SimpleError::new(format!("wrong checksum: {}, calculated {}", stored_checksum, checksum)));
    }

    let backup_file_header = reader.read_struct::<ese_db::FileHeader>(db_file_header.page_size as u64);

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

pub fn load_page_header(
    reader: &Reader,
    page_number: u32,
) -> Result<PageHeader, SimpleError> {
    let page_offset = ((page_number + 1) * (reader.page_size)) as u64;

    if reader.format_revision < ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT {
        let header = reader.read_struct::<PageHeaderOld>(page_offset);
        let common = reader.read_struct::<PageHeaderCommon>(page_offset + mem::size_of_val(&header) as u64);

        //let TODO_checksum = 0;
        Ok(PageHeader::old(header, common))
    } else if reader.format_revision < ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER {
        let header = reader.read_struct::<PageHeader0x0b>(page_offset);
        let common = reader.read_struct::<PageHeaderCommon>(page_offset + mem::size_of_val(&header) as u64);

        //TODO: verify checksum
        Ok(PageHeader::x0b(header, common))
    } else {
        let header = reader.read_struct::<PageHeader0x11>(page_offset);
        let common = reader.read_struct::<PageHeaderCommon>(page_offset + mem::size_of_val(&header) as u64);

        //TODO: verify checksum
        if reader.page_size > 8 * 1024 {
            let offs = mem::size_of_val(&header) + mem::size_of_val(&common);
            let ext = reader.read_struct::<PageHeaderExt0x11>(page_offset + offs as u64);

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
    let page_offset = ((db_page.page_number + 1) * reader.page_size) as u64;
    let mut tags_offset = (page_offset + reader.page_size as u64) as u64;
    let mut tags = Vec::<PageTag>::new();

    for _i in 0..db_page.get_available_page_tag() {
        tags_offset -= 2;
        let page_tag_offset : u16 = reader.read_struct(tags_offset);
        tags_offset -= 2;
        let page_tag_size : u16 = reader.read_struct(tags_offset);

        let mut flags : u8 = 0;
		let mut offset : u16 = 0;
        let mut size : u16 = 0;

        if reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER && reader.page_size >= 16384 {
			offset = page_tag_offset & 0x7fff;
            size   = page_tag_size & 0x7fff;

            // The upper 3-bits of the first 16-bit-value in the leaf page entry contain the page tag flags
            //if db_page.flags().contains(jet::PageFlags::IS_LEAF)
            {
                let flags_offset = page_offset + db_page.size() as u64 + offset as u64;
                let f : u16 = reader.read_struct(flags_offset);
                flags = (f >> 13) as u8;
            }
        } else {
            flags  = (page_tag_offset >> 13) as u8;
            offset = page_tag_offset & 0x1fff;
            size   = page_tag_size & 0x1fff;
        }
        tags.push(PageTag{ flags: flags, offset: offset, size: size} );
    }

    Ok(tags)
}

pub fn load_root_page_header(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
) -> Result<RootPageHeader, SimpleError> {
    let page_offset = ((db_page.page_number + 1) * reader.page_size) as u64;
    let root_page_offset = page_offset + db_page.size() as u64 + page_tag.offset as u64;

    // TODO Seen in format version 0x620 revision 0x14
    // check format and revision
    if page_tag.size == 16 {
        let root_page_header = reader.read_struct::<ese_db::RootPageHeader16>(root_page_offset);
        return Ok(RootPageHeader::xf(root_page_header));
    } else if page_tag.size == 25 {
        let root_page_header = reader.read_struct::<ese_db::RootPageHeader25>(root_page_offset);
        return Ok(RootPageHeader::x19(root_page_header));
    }

    Err(SimpleError::new(format!("wrong size of page tag: {:?}", page_tag)))
}

pub fn page_tag_get_branch_child_page_number(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
) -> Result<u32, SimpleError> {
    let page_offset = ((db_page.page_number + 1) * reader.page_size) as u64;
    let mut offset = page_offset + db_page.size() as u64 + page_tag.offset as u64;

    let mut common_page_key_size : u16 = 0;
    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        common_page_key_size = reader.read_struct(offset);
        offset += 2;
    }
    let local_page_key_size : u16 = reader.read_struct(offset);
    offset += 2;
    offset += local_page_key_size as u64;

    let child_page_number : u32 = reader.read_struct(offset);
    offset += 4;

    Ok(child_page_number)
}

pub fn load_catalog(
    reader: &mut Reader,
) -> Result<Vec<jet::TableDefinition>, SimpleError> {
    let db_page = jet::DbPage::new(reader, jet::FixedPageNumber::Catalog as u32)?;
    let pg_tags = &db_page.page_tags;

    if !db_page.flags().contains(jet::PageFlags::IS_PARENT) {
        return Err(SimpleError::new(format!("pageno {}: IS_PARENT (branch) flag should be present",
            db_page.page_number)));
    }

    let is_root = db_page.flags().contains(jet::PageFlags::IS_ROOT);

    if is_root {
        let root_page_header = load_root_page_header(reader, &db_page, &pg_tags[0]).unwrap();
        //println!("root_page {:?}", root_page_header);
    }

    let mut res : Vec<jet::TableDefinition> = vec![];
    let mut table_def : jet::TableDefinition = jet::TableDefinition { table_catalog_definition: None,
        column_catalog_definition_array: vec![], long_value_catalog_definition: None };

    let mut prev_page_number = db_page.page_number;
    let mut page_number = page_tag_get_branch_child_page_number(reader, &db_page, &pg_tags[1])?;
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

        for i in 1..pg_tags.len() {
            if jet::PageTagFlags::from_bits_truncate(pg_tags[i].flags).intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT) {
                continue;
            }
            let cat_item = load_catalog_item(reader, &db_page, &pg_tags[i])?;
            if cat_item.cat_type == jet::CatalogType::Table as u16 {
                if table_def.table_catalog_definition.is_some() {
                    res.push(table_def);
                    table_def = jet::TableDefinition { table_catalog_definition: None,
                        column_catalog_definition_array: vec![], long_value_catalog_definition: None };
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
    reader: &mut Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
) -> Result<jet::CatalogDefinition, SimpleError> {
    let page_offset = ((db_page.page_number + 1) * reader.page_size) as u64;
    let mut offset = page_offset + db_page.size() as u64 + page_tag.offset as u64;
    let offset_start = offset;

    let mut first_word_readed = false;
    let mut common_page_key_size : u16 = 0;
    let data: u16 = reader.read_struct(offset);
    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        common_page_key_size = clean_pgtag_flag(reader, db_page, data);
        first_word_readed = true;
        offset += 2;
    }
    let mut local_page_key_size : u16 = reader.read_struct(offset);
    if !first_word_readed {
        local_page_key_size = clean_pgtag_flag(reader, db_page, local_page_key_size);
        first_word_readed = true;
    }
    offset += 2;
    offset += local_page_key_size as u64;

    let data_left_in_tag = page_tag.size as u64 - (offset - offset_start);

    let offset_ddh = offset;
    let ddh = reader.read_struct::<ese_db::DataDefinitionHeader>(offset_ddh);
    offset += mem::size_of::<ese_db::DataDefinitionHeader>() as u64;

    let mut number_of_variable_size_data_types : u32 = 0;
    if ddh.last_variable_size_data_type > 127 {
        number_of_variable_size_data_types = ddh.last_variable_size_data_type as u32 - 127;
    }

    let cat_def_zero = std::mem::MaybeUninit::<jet::CatalogDefinition>::zeroed();
    let mut cat_def = unsafe { cat_def_zero.assume_init() };
    let data_def = reader.read_struct::<ese_db::DataDefinition>(offset);

    cat_def.father_data_page_object_identifier = data_def.father_data_page_object_identifier;
    cat_def.cat_type = data_def.data_type;
    cat_def.identifier = data_def.identifier;
    if cat_def.cat_type == jet::CatalogType::Column as u16 {
        cat_def.column_type = unsafe { data_def.coltyp_or_fdp.column_type };
    } else {
        cat_def.father_data_page_number = unsafe { data_def.coltyp_or_fdp.father_data_page_number };
    }
    cat_def.size = data_def.space_usage;
    // data_def.flags?
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
        let remaining_data_size = page_tag.size as u32 - variable_size_data_type_value_data_offset;
        for variable_size_data_type_iterator in 0..number_of_variable_size_data_types {
            offset += ddh.variable_size_data_types_offset as u64;
            let variable_size_data_type_size : u16 = reader.read_struct(offset_ddh + variable_size_data_types_offset as u64);
            variable_size_data_types_offset += 2;

            let mut data_type_size : u16 = 0;
            if variable_size_data_type_size & 0x8000 != 0 {
                data_type_size = 0;
            } else {
                data_type_size = variable_size_data_type_size - previous_variable_size_data_type_size;
            }
            if data_type_size > 0 {
                match data_type_number {
                    128 => {
                        let offset_dtn = offset_ddh + variable_size_data_type_value_data_offset as u64 + previous_variable_size_data_type_size as u64;
                        cat_def.name = reader.read_string(offset_dtn, data_type_size as usize);
                        //println!("cat_def.name: {}", cat_def.name);
                    },
                    130 => {
                        // TODO template_name
                    },
                    131 => {
                        // TODO default_value
                        let offset_def = offset_ddh + variable_size_data_type_value_data_offset as u64 + previous_variable_size_data_type_size as u64;
                        cat_def.default_value = reader.read_bytes(offset_def, data_type_size as usize);
                    },
                    132 | // KeyFldIDs
                    133 | // VarSegMac
                    134 | // ConditionalColumns
                    135 | // TupleLimits
                    136   // Version
                        => {
                        // not usefull fields
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

pub fn find_first_leaf_page(reader: &Reader, page_number: u32)
    -> Result<u32, SimpleError> {
    let db_page = jet::DbPage::new(reader, page_number)?;
    if db_page.flags().contains(jet::PageFlags::IS_LEAF) {
        return Ok(page_number);
    }

    let pg_tags = &db_page.page_tags;
    let child_page_number = page_tag_get_branch_child_page_number(reader, &db_page, &pg_tags[1])?;
    return find_first_leaf_page(reader, child_page_number);
}

pub fn load_data(
    reader: &Reader,
    tbl_def: &jet::TableDefinition,
    lv_tags: &Vec<LV_tags>,
    db_page: &jet::DbPage,
    page_tag_index: usize,
    column_id: u32
) -> Result<Option<Vec<u8>>, SimpleError> {
    let pg_tags = &db_page.page_tags;

    let is_root = db_page.flags().contains(jet::PageFlags::IS_ROOT);
    if is_root {
        let _root_page_header = load_root_page_header(reader, &db_page, &pg_tags[0]).unwrap();
        //println!("root_page {:?}", _root_page_header);
    }

    if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
        return Err(SimpleError::new(format!("expected leaf page, page_flags 0x{:?}",
            db_page.flags())));
    }

    if page_tag_index == 0 || page_tag_index >= pg_tags.len() {
        return Err(SimpleError::new(format!("wrong page tag index: {}", page_tag_index)));
    }

    let page_tag = &pg_tags[page_tag_index];
    let page_offset = ((db_page.page_number + 1) * reader.page_size) as u64;
    let mut offset = page_offset + db_page.size() as u64 + page_tag.offset as u64;
    let offset_start = offset;

    let mut first_word_readed = false;
    let mut common_page_key_size : u16 = 0;
    let data: u16 = reader.read_struct(offset);
    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        common_page_key_size = clean_pgtag_flag(reader, &db_page, data);
        first_word_readed = true;
        offset += 2;
    }
    let mut local_page_key_size : u16 = reader.read_struct(offset);
    if !first_word_readed {
        local_page_key_size = clean_pgtag_flag(reader, &db_page, local_page_key_size);
        first_word_readed = true;
    }
    offset += 2;
    offset += local_page_key_size as u64;

    let record_data_size = page_tag.size as u64 - (offset - offset_start);

    let offset_ddh = offset;
    let ddh = reader.read_struct::<ese_db::DataDefinitionHeader>(offset_ddh);
    offset += mem::size_of::<ese_db::DataDefinitionHeader>() as u64;

    let mut tagged_data_types_format = jet::TaggedDataTypesFormats::Index;
    if reader.format_version == 0x620 && reader.format_revision <= 2 {
        tagged_data_types_format = jet::TaggedDataTypesFormats::Linear;
    }

    let mut tagged_data_type_offset_bitmask : u16 = 0x3fff;
    if reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER && reader.page_size >= 16384 {
        tagged_data_type_offset_bitmask = 0x7fff;
    }

    let mut tagged_data_type_identifier : u16 = 0;
    let mut tagged_data_types_offset : u16 = 0;
    let mut tagged_data_type_offset : u16 = 0;
    let mut tagged_data_type_offset_data_size : u16 = 0;
    let mut previous_tagged_data_type_offset : u16 = 0;
    let mut remaining_definition_data_size : u16 = 0;
    let mut masked_previous_tagged_data_type_offset : u16 = 0;
    let mut previous_variable_size_data_type_size : u16 = 0;

    let mut number_of_variable_size_data_types : u16 = 0;
    if ddh.last_variable_size_data_type > 127 {
        number_of_variable_size_data_types = ddh.last_variable_size_data_type as u16 - 127;
    }

    let mut current_variable_size_data_type : u32 = 127;
    let mut variable_size_data_type_offset = ddh.variable_size_data_types_offset;
    let mut variable_size_data_type_value_offset : u16 = (ddh.variable_size_data_types_offset + (number_of_variable_size_data_types * 2)).try_into().unwrap();
    for j in 0..tbl_def.column_catalog_definition_array.len() {
        let col = &tbl_def.column_catalog_definition_array[j];
        if col.identifier <= 127 {
            if col.identifier <= ddh.last_fixed_size_data_type as u32 {
                // fixed size column
                if col.identifier == column_id {
                    let v = reader.read_bytes(offset, col.size as usize);
                    return Ok(Some(v));
                }
                offset += col.size as u64;
            } else if col.identifier == column_id {
                // no value in tag
                return Ok(None);
            }
        } else if current_variable_size_data_type < ddh.last_variable_size_data_type as u32 {
            // variable size
            while current_variable_size_data_type < col.identifier {
                let variable_size_data_type_size : u16 = reader.read_struct(offset_ddh + variable_size_data_type_offset as u64);
                variable_size_data_type_offset += 2;
                current_variable_size_data_type += 1;
                if current_variable_size_data_type == col.identifier {
                    if (variable_size_data_type_size & 0x8000) == 0 {

                        if col.identifier == column_id {
                            let v = reader.read_bytes(offset_ddh + variable_size_data_type_value_offset as u64,
                                (variable_size_data_type_size - previous_variable_size_data_type_size) as usize);
                            return Ok(Some(v));
                        }

                        variable_size_data_type_value_offset += variable_size_data_type_size - previous_variable_size_data_type_size;
                        previous_variable_size_data_type_size = variable_size_data_type_size;
                    }
                }
                if current_variable_size_data_type >= ddh.last_variable_size_data_type as u32 {
                    break;
                }
            }
        } else {
            // tagged
            if tagged_data_types_format == jet::TaggedDataTypesFormats::Linear {
                // TODO
                println!("TODO tagged_data_types_format == jet::TaggedDataTypesFormats::Linear");
            } else if tagged_data_types_format == jet::TaggedDataTypesFormats::Index {
                if tagged_data_types_offset == 0 {
                    tagged_data_types_offset = variable_size_data_type_value_offset;
                    remaining_definition_data_size = ((record_data_size - tagged_data_types_offset as u64) as u16).try_into().unwrap();

                    offset = offset_ddh + tagged_data_types_offset as u64;

                    if remaining_definition_data_size > 0 {
                        tagged_data_type_identifier = reader.read_struct::<u16>(offset);
                        offset += 2;

                        tagged_data_type_offset = reader.read_struct::<u16>(offset);
                        offset += 2;

                        if tagged_data_type_offset == 0 {
                            return Err(SimpleError::new("tagged_data_type_offset == 0"));
                        }
                        tagged_data_type_offset_data_size = (tagged_data_type_offset & 0x3fff) - 4;
                        remaining_definition_data_size -= 4;
                    }
                }
                if remaining_definition_data_size > 0 && col.identifier == tagged_data_type_identifier as u32 {
                    previous_tagged_data_type_offset = tagged_data_type_offset;
                    if tagged_data_type_offset_data_size > 0 {
                        tagged_data_type_identifier = reader.read_struct::<u16>(offset);
                        offset += 2;

                        tagged_data_type_offset = reader.read_struct::<u16>(offset);
                        offset += 2;

                        tagged_data_type_offset_data_size -= 4;
                        remaining_definition_data_size    -= 4;
                    }

                    masked_previous_tagged_data_type_offset = previous_tagged_data_type_offset & tagged_data_type_offset_bitmask;
                    let masked_tagged_data_type_offset = tagged_data_type_offset & tagged_data_type_offset_bitmask;

                    let mut tagged_data_type_size = 0;
                    if masked_tagged_data_type_offset > masked_previous_tagged_data_type_offset {
                        tagged_data_type_size = masked_tagged_data_type_offset - masked_previous_tagged_data_type_offset;
                    } else {
                        tagged_data_type_size = remaining_definition_data_size;
                    }
                    let mut tagged_data_type_value_offset = tagged_data_types_offset + masked_previous_tagged_data_type_offset;
                    let mut data_type_flags : u8 = 0;
                    if tagged_data_type_size > 0 {
                        remaining_definition_data_size -= tagged_data_type_size;
                        if (reader.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER &&
                            reader.page_size >= 16384) || (previous_tagged_data_type_offset & 0x4000 ) != 0
                        {
                            data_type_flags = reader.read_struct::<u8>(offset_ddh + tagged_data_type_value_offset as u64);
                            // 5 = VARIABLE_SIZE(0x1) & LONG_VALUE(0x4)
                            if data_type_flags != 0 && data_type_flags != 5 && data_type_flags != 1 {
                                let dtf = jet::TaggedDataTypeFlag::from_bits(data_type_flags.into());
                                println!("{} unsupported data type flags: {:?}", col.name, dtf);
                            }
                            tagged_data_type_value_offset += 1;
                            tagged_data_type_size         -= 1;
                        }
                    }
                    if tagged_data_type_size > 0 && col.identifier == column_id {
                        if jet::TaggedDataTypeFlag::from_bits_truncate(data_type_flags as u16).intersects(jet::TaggedDataTypeFlag::LONG_VALUE) {
                            let key = reader.read_struct::<u32>(offset_ddh + tagged_data_type_value_offset as u64);
                            let v = load_lv_data(reader, &lv_tags, key).unwrap();
                            return Ok(Some(v));
                        } else {
                            let v = reader.read_bytes(offset_ddh + tagged_data_type_value_offset as u64,
                                tagged_data_type_size as usize);
                            return Ok(Some(v));
                        }
                    }
                }
            }
        }
        if col.identifier == column_id {
            // column not found (empty)
            return Ok(None)
        }
    }

    Err(SimpleError::new(format!("column {} not found", column_id)))
}

pub fn load_lv_tag(
    reader: &Reader,
    db_page: &jet::DbPage,
    page_tag: &PageTag,
    page_tag_0: &PageTag
) -> Result<Option<LV_tags>, SimpleError> {
    let page_offset = ((db_page.page_number + 1) * reader.page_size) as u64;
    let mut offset = page_offset + db_page.size() as u64 + page_tag.offset as u64;
    let page_tag_offset : u64 = offset;

    let mut res = LV_tags { common_page_key: vec![], local_page_key: vec![], key: 0, offset: 0, size: 0, seg_offset: 0 };

    let mut first_word_readed = false;
    let mut common_page_key_size : u16 = 0;
    if page_tag.flags().intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE) {
        common_page_key_size = clean_pgtag_flag(reader, db_page, reader.read_struct::<u16>(offset));
        first_word_readed = true;
        offset += 2;

        if common_page_key_size > 0 {
            let offset0 = page_offset + db_page.size() as u64 + page_tag_0.offset as u64;
            let mut common_page_key = reader.read_bytes(offset0, common_page_key_size as usize);
            res.common_page_key.append(&mut common_page_key);
        }
    }

    let mut local_page_key_size : u16 = reader.read_struct(offset);
    if !first_word_readed {
        local_page_key_size = clean_pgtag_flag(reader, db_page, local_page_key_size);
        first_word_readed = true;
    }
    offset += 2;
    if local_page_key_size > 0 {
        let mut local_page_key = reader.read_bytes(offset, local_page_key_size as usize);
        res.local_page_key.append(&mut local_page_key);
        offset += local_page_key_size as u64;
    }

    if (page_tag.size as u64) - (offset - page_tag_offset) == 8 {
        let skey : u32 = reader.read_struct(offset);
        offset += 4;

        res.key = skey;

        let total_size : u32 = reader.read_struct(offset);
        offset += 4;

        // TODO: handle? page_tags with skey & total_size only
        return Ok(None);
    } else {

        let mut page_key : Vec<u8> = vec![];
        if common_page_key_size + local_page_key_size == 8 {
            page_key.append(&mut res.common_page_key.clone());
            page_key.append(&mut res.local_page_key.clone());
        } else if local_page_key_size >= 4 {
            page_key = res.local_page_key.clone();
        } else if common_page_key_size >= 4 {
            page_key = res.common_page_key.clone();
        }
        
        let skey = unsafe { std::mem::transmute::<[u8; 4], u32>(page_key[0..4].try_into().unwrap()) }.to_be();

        res.key = skey;

        if page_key.len() == 8 {
            let segment_offset = unsafe { std::mem::transmute::<[u8; 4], u32>(page_key[4..8].try_into().unwrap()) }.to_be();
            res.seg_offset = segment_offset;
        }

        res.offset = offset;
        res.size = (page_tag.size as u64 - (offset - page_tag_offset)).try_into().unwrap();

        return Ok(Some(res));
    }
}

// TODO: change to map[key] = vec![], sorted by seg_offset
#[derive(Debug, Clone)]
pub struct LV_tags {
    pub common_page_key: Vec<u8>,
    pub local_page_key: Vec<u8>,
    pub key: u32,
    pub offset: u64,
    pub size: u32,
    pub seg_offset: u32,
}

impl std::fmt::Display for LV_tags{
    fn fmt (&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "common_page_key {:?}, local_page_key {:?}, key {}, offset {}, seg_offset {}, size {}",
            self.common_page_key, self.local_page_key, self.key, self.offset, self.seg_offset, self.size)
    }
}

pub fn load_lv_metadata(
    reader: &Reader,
    page_number: u32
) -> Result<Vec<LV_tags>, SimpleError> {
    let db_page = jet::DbPage::new(reader, page_number)?;
    let pg_tags = &db_page.page_tags;

    if !db_page.flags().contains(jet::PageFlags::IS_LONG_VALUE) {
        return Err(SimpleError::new(format!("pageno {}: IS_LONG_VALUE flag should be present",
            db_page.page_number)));
    }

    let is_root = db_page.flags().contains(jet::PageFlags::IS_ROOT);
    if is_root {
        let _root_page_header = load_root_page_header(reader, &db_page, &pg_tags[0]).unwrap();
        //println!("root_page {:?}", _root_page_header);
    }

    let mut res : Vec<LV_tags> = vec![];

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
                let mut r = load_lv_metadata(reader, page_number);
                match r {
                    Ok(mut r) => {
                        res.append(&mut r);
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
                            if r.is_some() {
                                res.push(r.unwrap());
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
                    if r.is_some() {
                        res.push(r.unwrap());
                    }
                },
                Err(e) => return Err(e)
            }
        }
    }

    Ok(res)
}

pub fn load_lv_data(
    reader: &Reader,
    lv_tags: &Vec<LV_tags>,
    long_value_key: u32,
) -> Result<Vec<u8>, SimpleError> {
    let mut res : Vec<u8> = vec![];
    let mut i = 0;
    while i < lv_tags.len() {
        if long_value_key == lv_tags[i].key && res.len() == lv_tags[i].seg_offset as usize {
            let mut v = reader.read_bytes(lv_tags[i].offset, lv_tags[i].size as usize);
            res.append(&mut v);
            i = 0; // search next seg_offset (lv_tags could be not sorted)
            continue;
        }
        i += 1;
    }

    if res.len() > 0 {
        Ok(res)
    } else {
        Err(SimpleError::new(format!("LV key {} not found", long_value_key)))
    }
}