//reader.rs
use cache_2q::Cache;
use simple_error::SimpleError;
use std::{
    array::TryFromSliceError,
    cell::RefCell,
    collections::{hash_map::Entry, BTreeSet, HashMap},
    convert::TryInto,
    io,
    io::{Read, Seek},
    mem,
};

use crate::parser::decomp::*;
use crate::parser::ese_db;
use crate::parser::ese_db::*;
use crate::parser::jet;
use crate::utils::*;

// #[cfg(all(feature = "nt_comparison", target_os = "windows"))]
// mod gen_db;
//
// mod test;

pub trait ReadSeek: Read + Seek {
    fn tell(&mut self) -> io::Result<u64> {
        self.stream_position()
    }
}

impl<T: Read + Seek> ReadSeek for T {}

pub struct Reader<T: ReadSeek> {
    file: RefCell<T>,
    cache: RefCell<Cache<u32, Vec<u8>>>,
    format_version: jet::FormatVersion,
    format_revision: jet::FormatRevision,
    page_size: u32,
    pub db_state: jet::DbState,
}

impl<T: ReadSeek> Reader<T> {
    fn is_small_page(&self) -> bool {
        self.page_size <= 1024 * 8
    }

    fn load_db_file_header(&mut self) -> Result<ese_db::FileHeader, SimpleError> {
        let (mut db_file_header, _) = ese_db::FileHeader::read(self, 0)?;

        if db_file_header.signature != ESEDB_FILE_SIGNATURE {
            return Err(SimpleError::new("bad file_header.signature"));
        }

        let (backup_file_header, _) =
            ese_db::FileHeader::read(self, db_file_header.page_size as u64)?;

        if db_file_header.format_revision == 0 {
            db_file_header.format_revision = backup_file_header.format_revision;
        }

        if db_file_header.format_revision != backup_file_header.format_revision {
            return Err(SimpleError::new(format!(
                "mismatch in format revision: {} not equal to backup value {}",
                db_file_header.format_revision, backup_file_header.format_revision
            )));
        }

        if db_file_header.page_size == 0 {
            db_file_header.page_size = backup_file_header.page_size;
        }

        if db_file_header.page_size != backup_file_header.page_size {
            return Err(SimpleError::new(format!(
                "mismatch in page size: {} not equal to backup value {}",
                db_file_header.page_size, backup_file_header.page_size
            )));
        }
        if db_file_header.format_version != 0x620 {
            return Err(SimpleError::new(format!(
                "unsupported format version: {}",
                db_file_header.format_version
            )));
        }

        self.page_size = db_file_header.page_size;
        // reset cache (cache size changed)
        self.cache.borrow_mut().clear();

        let mut file_header_page = vec![0u8; self.page_size as usize];
        self.read(0, &mut file_header_page)?;
        let checksum = calc_crc32(&file_header_page);
        let stored_checksum = db_file_header.checksum;
        if stored_checksum != checksum {
            return Err(SimpleError::new(format!(
                "wrong checksum: {}, calculated {}",
                stored_checksum, checksum
            )));
        }

        Ok(db_file_header)
    }

    pub fn new(read_seek: T, cache_size: usize) -> Result<Reader<T>, SimpleError> {
        let mut reader = Reader {
            file: RefCell::new(read_seek),
            cache: RefCell::new(Cache::new(cache_size)),
            page_size: 2 * 1024, //just to read header
            format_version: 0,
            format_revision: 0,
            db_state: jet::DbState::impossible,
        };

        let db_fh = reader.load_db_file_header()?;
        reader.format_version = db_fh.format_version;
        reader.format_revision = db_fh.format_revision;
        reader.page_size = db_fh.page_size;
        reader.db_state = db_fh.database_state;
        reader.cache.get_mut().clear();

        Ok(reader)
    }

    pub fn is_dirty(&self) -> bool {
        return self.db_state == jet::DbState::DirtyShutdown;
    }

    pub fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), SimpleError> {
        if buf.len() > self.page_size as usize {
            return Err(SimpleError::new(format!(
                "Attempting to read {} bytes at offset {}, more than page_size ({})",
                buf.len(),
                offset,
                self.page_size
            )));
        }
        let pg_no = (offset / self.page_size as u64) as u32;
        let mut c = self.cache.borrow_mut();
        if !c.contains_key(&pg_no) {
            let mut page_buf = vec![0u8; self.page_size as usize];
            let f = &mut self.file.borrow_mut();
            match f.seek(io::SeekFrom::Start(pg_no as u64 * self.page_size as u64)) {
                Ok(_) => match f.read_exact(&mut page_buf) {
                    Ok(_) => {
                        c.insert(pg_no, page_buf);
                    }
                    Err(e) => {
                        return Err(SimpleError::new(format!("read_exact failed: {:?}", e)));
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
            }
            None => {
                return Err(SimpleError::new(format!(
                    "Cache failed, page number not found: {}",
                    pg_no
                )));
            }
        }

        Ok(())
    }

    pub fn read_bytes(&self, offset: u64, size: usize) -> Result<Vec<u8>, SimpleError> {
        let mut buf = vec![0u8; size];
        let mut readed = 0;
        while readed != size {
            let read_size = std::cmp::min(self.page_size as usize, size - readed);
            self.read(offset + readed as u64, &mut buf[readed..readed + read_size])?;
            readed += read_size;
        }
        Ok(buf)
    }

    pub fn read_string(&self, offset: u64, size: usize) -> Result<String, SimpleError> {
        let v = self.read_bytes(offset, size)?;
        match std::str::from_utf8(&v) {
            Ok(s) => Ok(s.to_string()),

            Err(e) => Err(SimpleError::new(format!(
                "from_utf8 failed: error_len() is {:?}",
                e.error_len()
            ))),
        }
    }

    pub fn load_db(read_seek: T, cache_size: usize) -> Result<Reader<T>, SimpleError> {
        Reader::new(read_seek, cache_size)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn cache(&self) -> RefCell<Cache<u32, Vec<u8>>> {
        self.cache.clone()
    }

    pub(crate) fn load_page_header(&self, page_number: u32) -> Result<PageHeader, SimpleError> {
        let page_offset = (page_number + 1) as u64 * (self.page_size) as u64;
        let page_data = self.read_bytes(page_offset, self.page_size as usize)?;

        fn calc_new_checksum_cmp_with(
            buffer: &[u8],
            page_number: u32,
            checksum: u64,
            skip_header: bool,
        ) -> Result<(), SimpleError> {
            let calc_checksum: u64 = calc_new_crc(buffer, page_number, skip_header)?;
            if calc_checksum != checksum {
                return Err(SimpleError::new(format!(
                            "Page number: {}, calculated checksum 0x{:X} doesn't equal stored page header checksum 0x{:X}",
                            page_number, calc_checksum, checksum)));
            }
            Ok(())
        }

        let checksum = read_u64(self, page_offset)?;
        let page_flags = read_u32(self, page_offset + 36)?;
        if jet::PageFlags::from_bits_truncate(page_flags)
            .intersects(jet::PageFlags::IS_NEW_RECORD_FORMAT)
        {
            let mut block_len = page_data.len();
            if !self.is_small_page() {
                block_len = page_data.len() / 4;
                let ext = PageHeaderExt0x11::read(self, page_offset + 40)?;

                if ext.page_number != page_number as u64 {
                    return Err(SimpleError::new(format!(
                            "Page number: {} doesn't equal to stored page number {} in extended page header",
                            page_number, {ext.page_number})));
                }

                calc_new_checksum_cmp_with(
                    &page_data[block_len..block_len * 2],
                    page_number,
                    ext.checksum1,
                    false,
                )?;
                calc_new_checksum_cmp_with(
                    &page_data[block_len * 2..block_len * 3],
                    page_number,
                    ext.checksum2,
                    false,
                )?;
                calc_new_checksum_cmp_with(
                    &page_data[block_len * 3..],
                    page_number,
                    ext.checksum3,
                    false,
                )?;
            }
            calc_new_checksum_cmp_with(&page_data[..block_len], page_number, checksum, true)?;
        } else {
            let calc_checksum = ((page_number as u64) << 32) | (calc_crc32(&page_data) as u64);
            if calc_checksum != checksum {
                return Err(SimpleError::new(format!(
                        "Page number: {}, calculated checksum 0x{:X} doesn't equal stored page header checksum 0x{:X}",
                        page_number, calc_checksum, checksum)));
            }
        }

        if self.format_revision < ESEDB_FORMAT_REVISION_NEW_RECORD_FORMAT {
            let header = PageHeaderOld::read(self, page_offset)?;
            let common =
                PageHeaderCommon::read(self, page_offset + mem::size_of_val(&header) as u64)?;
            Ok(PageHeader::old(header, common))
        } else if self.format_revision < ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER {
            let header = PageHeader0x0b::read(self, page_offset)?;
            let common =
                PageHeaderCommon::read(self, page_offset + mem::size_of_val(&header) as u64)?;
            Ok(PageHeader::x0b(header, common))
        } else {
            let header = PageHeader0x11::read(self, page_offset)?;
            let common =
                PageHeaderCommon::read(self, page_offset + mem::size_of_val(&header) as u64)?;
            if !self.is_small_page() {
                let offs = mem::size_of_val(&header) + mem::size_of_val(&common);
                let ext = PageHeaderExt0x11::read(self, page_offset + offs as u64)?;
                Ok(PageHeader::x11_ext(header, common, ext))
            } else {
                Ok(PageHeader::x11(header, common))
            }
        }
    }

    pub fn load_page_tags(&self, db_page: &jet::DbPage) -> Result<Vec<PageTag>, SimpleError> {
        let page_offset = db_page.offset();
        let mut tags_offset = page_offset + self.page_size as u64;
        let tags_cnt = db_page.get_available_page_tag();
        let mut tags = Vec::<PageTag>::with_capacity(tags_cnt);

        for _i in 0..tags_cnt {
            tags_offset -= 2;
            let page_tag_offset = read_u16(self, tags_offset)?;
            tags_offset -= 2;
            let page_tag_size = read_u16(self, tags_offset)?;

            let flags: u8;
            let offset: u16;
            let size: u16;

            if self.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER
                && self.page_size >= 16384
            {
                offset = page_tag_offset & 0x7fff;
                size = page_tag_size & 0x7fff;

                // The upper 3-bits of the first 16-bit-value in the leaf page entry contain the page tag flags
                //if db_page.flags().contains(jet::PageFlags::IS_LEAF)
                {
                    let flags_offset = page_offset + db_page.size() as u64 + offset as u64;
                    let f: u16 = read_u16(self, flags_offset)?;
                    flags = (f >> 13) as u8;
                }
            } else {
                flags = (page_tag_offset >> 13) as u8;
                offset = page_tag_offset & 0x1fff;
                size = page_tag_size & 0x1fff;
            }
            tags.push(PageTag {
                flags,
                offset,
                size,
            });
        }

        Ok(tags)
    }

    pub fn load_root_page_header(
        &self,
        db_page: &jet::DbPage,
        page_tag: &PageTag,
    ) -> Result<RootPageHeader, SimpleError> {
        let root_page_offset = page_tag.offset(db_page);

        // TODO Seen in format version 0x620 revision 0x14
        // check format and revision
        if page_tag.size == 16 {
            let root_page_header = ese_db::RootPageHeader16::read(self, root_page_offset)?;
            return Ok(RootPageHeader::xf(root_page_header));
        } else if page_tag.size == 25 {
            let root_page_header = ese_db::RootPageHeader25::read(self, root_page_offset)?;
            return Ok(RootPageHeader::x19(root_page_header));
        }

        Err(SimpleError::new(format!(
            "wrong size of page tag: {:?}",
            page_tag
        )))
    }

    pub fn page_tag_get_branch_child_page_number(
        &self,
        db_page: &jet::DbPage,
        page_tag: &PageTag,
    ) -> Result<u32, SimpleError> {
        let mut offset = page_tag.offset(db_page);

        if page_tag
            .flags()
            .intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE)
        {
            // Why is this intersect vs contains?
            offset += 2;
        }
        let local_page_key_size: u16 = read_u16(self, offset)?;
        offset += 2;
        offset += local_page_key_size as u64;

        let child_page_number: u32 = read_u32(self, offset)?;
        Ok(child_page_number)
    }

    pub fn load_catalog(&self) -> Result<Vec<jet::TableDefinition>, SimpleError> {
        let db_page = jet::DbPage::new(self, jet::FixedPageNumber::Catalog as u32)?;

        let is_root = db_page.flags().contains(jet::PageFlags::IS_ROOT);
        if is_root {
            let _root_page_header = self.load_root_page_header(&db_page, db_page.tag(0)?)?;
        }

        let mut res: Vec<jet::TableDefinition> = vec![];
        let mut table_def: jet::TableDefinition = jet::TableDefinition {
            table_catalog_definition: None,
            column_catalog_definition_array: vec![],
            long_value_catalog_definition: None,
        };

        let mut page_number;
        if db_page.flags().contains(jet::PageFlags::IS_PARENT) {
            page_number = self.page_tag_get_branch_child_page_number(&db_page, db_page.tag(1)?)?;
        } else if db_page.flags().contains(jet::PageFlags::IS_LEAF) {
            page_number = db_page.page_number;
        } else {
            return Err(SimpleError::new(format!(
                "pageno {}: neither IS_PARENT nor IS_LEAF is present in {:?}",
                db_page.page_number,
                db_page.flags()
            )));
        }
        let mut prev_page_number = db_page.page_number;

        while page_number != 0 {
            let db_page = jet::DbPage::new(self, page_number)?;

            if db_page.prev_page() != 0 && prev_page_number != db_page.prev_page() {
                return Err(SimpleError::new(format!(
                    "pageno {}: wrong previous_page number {}, expected {}",
                    db_page.page_number,
                    db_page.prev_page(),
                    prev_page_number
                )));
            }
            if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
                return Err(SimpleError::new(format!(
                    "pageno {}: IS_LEAF flag should be present",
                    db_page.page_number
                )));
            }

            for i in 1..db_page.tags() {
                let pg_tag = db_page.tag(i)?;
                if jet::PageTagFlags::from_bits_truncate(pg_tag.flags)
                    .intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT)
                {
                    continue;
                }
                let cat_item = self.load_catalog_item(&db_page, pg_tag)?;
                if cat_item.cat_type == jet::CatalogType::Table as u16 {
                    if table_def.table_catalog_definition.is_some() {
                        res.push(table_def);
                        table_def = jet::TableDefinition {
                            table_catalog_definition: None,
                            column_catalog_definition_array: vec![],
                            long_value_catalog_definition: None,
                        };
                    } else if !table_def.column_catalog_definition_array.is_empty()
                        || table_def.long_value_catalog_definition.is_some()
                    {
                        return Err(SimpleError::new(
                            "corrupted table detected: column/long definition is going before table"));
                    }
                    table_def.table_catalog_definition = Some(cat_item);
                } else if cat_item.cat_type == jet::CatalogType::Column as u16 {
                    table_def.column_catalog_definition_array.push(cat_item);
                } else if cat_item.cat_type == jet::CatalogType::LongValue as u16 {
                    if table_def.long_value_catalog_definition.is_some() {
                        return Err(SimpleError::new("long-value catalog definition duplicate?"));
                    }
                    table_def.long_value_catalog_definition = Some(cat_item);
                }
                // we knowingly ignore Index and Callback Catalog types
                else if cat_item.cat_type != jet::CatalogType::Index as u16
                    && cat_item.cat_type != jet::CatalogType::Callback as u16
                {
                    return Err(SimpleError::new(format!(
                        "TODO: Unhandled cat_item.cat_type {}",
                        cat_item.cat_type
                    )));
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
        &self,
        db_page: &jet::DbPage,
        page_tag: &PageTag,
    ) -> Result<jet::CatalogDefinition, SimpleError> {
        let mut offset = page_tag.offset(db_page);

        let mut first_word_read = false;
        if page_tag
            .flags()
            .intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE)
        {
            first_word_read = true;
            offset += 2;
        }
        let mut local_page_key_size: u16 = read_u16(self, offset)?;
        if !first_word_read {
            local_page_key_size = self.clean_pgtag_flag(db_page, local_page_key_size);
        }
        offset += 2;
        offset += local_page_key_size as u64;

        let offset_ddh = offset;
        let ddh = ese_db::DataDefinitionHeader::read(self, offset_ddh)?;
        offset += mem::size_of::<ese_db::DataDefinitionHeader>() as u64;

        let number_of_variable_size_data_types: u32 = if ddh.last_variable_size_data_type > 127 {
            ddh.last_variable_size_data_type as u32 - 127
        } else {
            0
        };

        let mut cat_def = jet::CatalogDefinition::default();
        let data_def = ese_db::DataDefinition::read(self, offset)?;

        cat_def.father_data_page_object_identifier = data_def.father_data_page_object_identifier;
        cat_def.cat_type = data_def.data_type;
        cat_def.identifier = data_def.identifier;
        if cat_def.cat_type == jet::CatalogType::Column as u16 {
            cat_def.column_type = data_def.coltyp_or_fdp.column_type();
        } else {
            cat_def.father_data_page_number = data_def.coltyp_or_fdp.father_data_page_number();
        }
        cat_def.size = data_def.space_usage;
        cat_def.flags = data_def.flags;
        if cat_def.cat_type == jet::CatalogType::Column as u16 {
            cat_def.codepage = data_def.pages_or_locale.codepage();
        }
        if ddh.last_fixed_size_data_type >= 10 {
            cat_def.lcmap_flags = data_def.lc_map_flags;
        }

        if number_of_variable_size_data_types > 0 {
            let mut variable_size_data_types_offset = ddh.variable_size_data_types_offset as u32;
            let variable_size_data_type_value_data_offset =
                variable_size_data_types_offset + (number_of_variable_size_data_types * 2);
            let mut previous_variable_size_data_type_size: u16 = 0;
            let mut data_type_number: u16 = 128;
            for _ in 0..number_of_variable_size_data_types {
                offset += ddh.variable_size_data_types_offset as u64;
                let variable_size_data_type_size: u16 =
                    read_u16(self, offset_ddh + variable_size_data_types_offset as u64)?;
                variable_size_data_types_offset += 2;

                let data_type_size: u16 = if variable_size_data_type_size & 0x8000 != 0 {
                    0
                } else {
                    variable_size_data_type_size - previous_variable_size_data_type_size
                };
                if data_type_size > 0 {
                    match data_type_number {
                        128 => {
                            let offset_dtn = offset_ddh + variable_size_data_type_value_data_offset as u64 + previous_variable_size_data_type_size as u64;
                            cat_def.name = self.read_string(offset_dtn, data_type_size as usize)?;
                        },
                        130 => {
                            // TODO template_name
                        },
                        131 => {
                            // TODO default_value
                            let offset_def = offset_ddh + variable_size_data_type_value_data_offset as u64 + previous_variable_size_data_type_size as u64;
                            cat_def.default_value = self.read_bytes(offset_def, data_type_size as usize)?;
                        },
                        132 | // KeyFldIDs
                        133 | // VarSegMac
                        134 | // ConditionalColumns
                        135 | // TupleLimits
                        136 | // Version
                        137  // iMSO_SortID (?)
                            => {
                            // not useful fields
                        },
                        _ => {
                            if data_type_size > 0 {
                                return Err(SimpleError::new(format!("TODO handle data_type_number: {}", data_type_number)));
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

    pub fn clean_pgtag_flag(&self, db_page: &jet::DbPage, data: u16) -> u16 {
        // The upper 3-bits of the first 16-bit-value in the leaf page entry contain the page tag flags
        if self.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER
            && self.page_size >= 16384
            && db_page.flags().contains(jet::PageFlags::IS_LEAF)
        {
            return data & 0x1FFF;
        }
        data
    }

    pub fn find_first_leaf_page(&self, mut page_number: u32) -> Result<u32, SimpleError> {
        let mut visited_pages: BTreeSet<u32> = BTreeSet::new();
        loop {
            if visited_pages.contains(&page_number) {
                return Err(SimpleError::new(format!(
                    "Child page loop detected at page number {}, visited pages: {:?}",
                    page_number, visited_pages
                )));
            }

            let db_page = jet::DbPage::new(self, page_number)?;
            if db_page.flags().contains(jet::PageFlags::IS_LEAF) {
                return Ok(page_number);
            } else {
                visited_pages.insert(page_number);
            }

            page_number = self.page_tag_get_branch_child_page_number(&db_page, db_page.tag(1)?)?;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn load_data(
        &self,
        lls: &mut LastLoadState,
        tbl_def: &jet::TableDefinition,
        lv_tags: &LV_tags,
        db_page: &jet::DbPage,
        page_tag_index: usize,
        column_id: u32,
        multi_value_index: usize, // 0 value mean itagSequence = 1
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
            return Err(SimpleError::new(format!(
                "expected leaf page, page_flags 0x{:?}",
                db_page.flags()
            )));
        }

        if page_tag_index == 0 {
            // this indicates an empty table; this is ok
            return Ok(None);
        }

        if page_tag_index >= db_page.tags() {
            return Err(SimpleError::new(format!(
                "wrong page tag index: {}",
                page_tag_index
            )));
        }

        let page_tag = db_page.tag(page_tag_index)?;

        let mut tagged_data_types_format = jet::TaggedDataTypesFormats::Index;
        if self.format_version == 0x620 && self.format_revision <= 2 {
            tagged_data_types_format = jet::TaggedDataTypesFormats::Linear;
        }

        let mut start_i = 0;
        if lls.last_column == 0 {
            lls.offset = page_tag.offset(db_page);
            let offset_start = lls.offset;

            let mut first_word_read = false;
            if page_tag
                .flags()
                .intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE)
            {
                first_word_read = true;
                lls.offset += 2;
            }
            let mut local_page_key_size: u16 = read_u16(self, lls.offset)?;
            if !first_word_read {
                local_page_key_size = self.clean_pgtag_flag(db_page, local_page_key_size);
            }
            lls.offset += 2;
            lls.offset += local_page_key_size as u64;

            lls.record_data_size = page_tag.size as u64 - (lls.offset - offset_start);

            lls.offset_ddh = lls.offset;
            lls.ddh = ese_db::DataDefinitionHeader::read(self, lls.offset_ddh)?;
            lls.offset += mem::size_of::<ese_db::DataDefinitionHeader>() as u64;

            // read fixed data bits mask, located at the end of fixed columns
            lls.fixed_data_bits_mask_size = (lls.ddh.last_fixed_size_data_type as usize + 7) / 8;
            if lls.fixed_data_bits_mask_size > 0 {
                lls.fixed_data_bits_mask = self.read_bytes(
                    lls.offset_ddh + lls.ddh.variable_size_data_types_offset as u64
                        - lls.fixed_data_bits_mask_size as u64,
                    lls.fixed_data_bits_mask_size,
                )?;
            }

            let number_of_variable_size_data_types: u16 =
                if lls.ddh.last_variable_size_data_type > 127 {
                    lls.ddh.last_variable_size_data_type as u16 - 127
                } else {
                    0
                };

            lls.var_state.current_type = 127;
            lls.var_state.type_offset = lls.ddh.variable_size_data_types_offset;
            lls.var_state.value_offset =
                lls.ddh.variable_size_data_types_offset + (number_of_variable_size_data_types * 2);
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
                        if lls.fixed_data_bits_mask_size > 0
                            && lls.fixed_data_bits_mask[i / 8] & (1 << (i % 8)) > 0
                        {
                            // empty value
                            return Ok(None);
                        }
                        let v = self.read_bytes(lls.offset, col.size as usize)?;
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
                    let variable_size_data_type_size: u16 =
                        read_u16(self, lls.offset_ddh + lls.var_state.type_offset as u64)?;
                    lls.var_state.type_offset += 2;
                    lls.var_state.current_type += 1;
                    if lls.var_state.current_type == col.identifier
                        && (variable_size_data_type_size & 0x8000) == 0
                    {
                        let var_offset = lls.offset_ddh + lls.var_state.value_offset as u64;
                        let var_size = variable_size_data_type_size
                            - lls.previous_variable_size_data_type_size;

                        lls.var_state.value_offset += var_size;
                        lls.previous_variable_size_data_type_size = variable_size_data_type_size;

                        if col.identifier == column_id {
                            let v = self.read_bytes(var_offset, var_size as usize)?;
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
                    println!(
                        "TODO tagged_data_types_format ==-- jet::TaggedDataTypesFormats::Linear"
                    );
                } else if tagged_data_types_format == jet::TaggedDataTypesFormats::Index {
                    match self.load_tagged_data_linear(
                        lv_tags,
                        col,
                        column_id,
                        &mut lls.tag_state,
                        &mut lls.var_state,
                        &mut lls.offset,
                        lls.offset_ddh,
                        lls.record_data_size,
                        multi_value_index,
                    ) {
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
                return Ok(None);
            }
        }

        Err(SimpleError::new(format!("column {} not found", column_id)))
    }

    fn init_tag_state(
        &self,
        tag_state: &mut TaggedDataState,
        var_state: VariableSizeDataState,
        offset: &mut u64,
        offset_ddh: u64,
        record_data_size: u64,
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        tag_state.types_offset = var_state.value_offset;

        tag_state.remaining_definition_data_size = (record_data_size
            - tag_state.types_offset as u64)
            .try_into()
            .map_err(|e: std::num::TryFromIntError| SimpleError::new(e.to_string()))?;

        *offset = offset_ddh + tag_state.types_offset as u64;

        if tag_state.remaining_definition_data_size > 0 {
            tag_state.identifier = read_u16(self, *offset)?;
            *offset += 2;

            tag_state.type_offset = read_u16(self, *offset)?;
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
        &self,
        lv_tags: &LV_tags,
        col: &jet::CatalogDefinition,
        column_id: u32,
        tag_state: &mut TaggedDataState,
        var_state: &mut VariableSizeDataState,
        offset: &mut u64,
        offset_ddh: u64,
        record_data_size: u64,
        multi_value_index: usize,
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        if tag_state.types_offset == 0 {
            self.init_tag_state(tag_state, *var_state, offset, offset_ddh, record_data_size)?;
        }
        if tag_state.remaining_definition_data_size > 0
            && col.identifier == tag_state.identifier as u32
        {
            let previous_tagged_data_type_offset = tag_state.type_offset;
            if tag_state.offset_data_size > 0 {
                tag_state.identifier = read_u16(self, *offset)?;
                *offset += 2;

                tag_state.type_offset = read_u16(self, *offset)?;
                *offset += 2;

                tag_state.offset_data_size -= 4;
                tag_state.remaining_definition_data_size -= 4;
            }

            let tagged_data_type_offset_bitmask: u16 = if self.format_revision
                >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER
                && self.page_size >= 16384
            {
                0x7fff
            } else {
                0x3fff
            };
            let masked_previous_tagged_data_type_offset: u16 =
                previous_tagged_data_type_offset & tagged_data_type_offset_bitmask;
            let masked_tagged_data_type_offset =
                tag_state.type_offset & tagged_data_type_offset_bitmask;

            if masked_tagged_data_type_offset > masked_previous_tagged_data_type_offset {
                tag_state.tagged_data_type_size =
                    masked_tagged_data_type_offset - masked_previous_tagged_data_type_offset;
            } else {
                tag_state.tagged_data_type_size = tag_state.remaining_definition_data_size;
            }
            let mut tagged_data_type_value_offset =
                tag_state.types_offset + masked_previous_tagged_data_type_offset;
            let mut data_type_flags: u8 = 0;
            if tag_state.tagged_data_type_size > 0 {
                tag_state.remaining_definition_data_size -= tag_state.tagged_data_type_size;
                if (self.format_revision >= ESEDB_FORMAT_REVISION_EXTENDED_PAGE_HEADER
                    && self.page_size >= 16384)
                    || (previous_tagged_data_type_offset & 0x4000) != 0
                {
                    data_type_flags =
                        read_u8(self, offset_ddh + tagged_data_type_value_offset as u64)?;

                    tagged_data_type_value_offset += 1;
                    tag_state.tagged_data_type_size -= 1;
                }
            }
            if tag_state.tagged_data_type_size > 0 && col.identifier == column_id {
                let value_offset = offset_ddh + tagged_data_type_value_offset as u64;
                match self.load_tagged_column(
                    lv_tags,
                    col,
                    value_offset,
                    tag_state.tagged_data_type_size,
                    data_type_flags,
                    multi_value_index,
                ) {
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

    fn read_lv_key(&self, offset: u64) -> Result<u64, SimpleError> {
        let mut bytes = self.read_bytes(offset, 4)?;
        // if fLID64 is set, this is LVKEY64
        let key = if bytes[3] & 0x80 > 0 {
            bytes.append(&mut self.read_bytes(offset + 4, 4)?);
            u64::from_bytes(&bytes)
        } else {
            u32::from_bytes(&bytes) as u64
        };
        Ok(key)
    }

    fn load_tagged_column(
        &self,
        lv_tags: &LV_tags,
        col: &jet::CatalogDefinition,
        offset: u64,
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
            v = self.load_lv_data(lv_tags, self.read_lv_key(offset)?, compressed)?;
        } else if dtf
            .intersects(TaggedDataTypeFlag::MULTI_VALUE | TaggedDataTypeFlag::MULTI_VALUE_OFFSET)
        {
            let mv = self.read_multi_value(
                offset,
                tagged_data_type_size,
                &dtf,
                multi_value_index,
                lv_tags,
                compressed,
            )?;
            if let Some(mv_data) = mv {
                v = mv_data;
            }
        } else if dtf.intersects(jet::TaggedDataTypeFlag::COMPRESSED) {
            v = self.read_bytes(offset, tagged_data_type_size as usize)?;
            let dsize = decompress_size(&v);
            if dsize > 0 {
                v = decompress_buf(&v, dsize)?;
            }
        } else {
            v = self.read_bytes(offset, tagged_data_type_size as usize)?;
        }

        if !v.is_empty() {
            return Ok(Some(v));
        }
        Ok(None)
    }

    fn read_multi_value(
        &self,
        offset: u64,
        tagged_data_type_size: u16,
        dtf: &jet::TaggedDataTypeFlag,
        multi_value_index: usize,
        lv_tags: &LV_tags,
        compressed: bool,
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        let mut mv_indexes: Vec<(u16 /*shift*/, (bool /*lv*/, u16 /*size*/))> = Vec::new();
        if dtf.intersects(jet::TaggedDataTypeFlag::MULTI_VALUE_OFFSET) {
            // The first byte contain the offset
            // [13, ...]
            let offset_mv_list = offset;
            let value: u16 = read_u8(self, offset_mv_list)? as u16;

            mv_indexes.push((1, (false, value)));
            mv_indexes.push((value + 1, (false, tagged_data_type_size - value - 1)));
        } else if dtf.intersects(jet::TaggedDataTypeFlag::MULTI_VALUE) {
            // The first 2 bytes contain the offset to the first value
            // there is an offset for every value
            // therefore first offset / 2 = the number of value entries
            // [8, 0, 7, 130, 11, 2, 10, 131, ...]
            let mut offset_mv_list = offset;
            let mut value = read_u16(self, offset_mv_list)?;
            offset_mv_list += 2;

            let mut value_entry_size: u16;
            let mut value_entry_offset = value & 0x7fff;
            let mut entry_lvbit: bool = (value & 0x8000) > 0;
            let number_of_value_entries = value_entry_offset / 2;

            for _ in 1..number_of_value_entries {
                value = read_u16(self, offset_mv_list)?;
                offset_mv_list += 2;
                value_entry_size = (value & 0x7fff) - value_entry_offset;
                mv_indexes.push((value_entry_offset, (entry_lvbit, value_entry_size)));
                entry_lvbit = (value & 0x8000) > 0;
                value_entry_offset = value & 0x7fff;
            }
            value_entry_size = tagged_data_type_size - value_entry_offset;
            mv_indexes.push((value_entry_offset, (entry_lvbit, value_entry_size)));
        } else {
            return Err(SimpleError::new(format!(
                "Unknown TaggedDataTypeFlag: {}",
                dtf.bits()
            )));
        }
        let mut mv_index = 0;
        if multi_value_index > 0 && multi_value_index - 1 < mv_indexes.len() {
            mv_index = multi_value_index - 1;
        }

        if mv_index < mv_indexes.len() {
            let (shift, (lv, size)) = mv_indexes[mv_index];
            let v;
            if lv {
                v = self.load_lv_data(
                    lv_tags,
                    self.read_lv_key(offset + shift as u64)?,
                    compressed,
                )?;
            } else {
                v = self.read_bytes(offset + shift as u64, size as usize)?;
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
        &self,
        db_page: &jet::DbPage,
        page_tag: &PageTag,
        page_tag_0: &PageTag,
    ) -> Result<Option<LV_tags>, SimpleError> {
        let mut offset = page_tag.offset(db_page);
        let page_tag_offset: u64 = offset;

        let mut res = LV_tag {
            common_page_key: vec![],
            local_page_key: vec![],
            offset: 0,
            size: 0,
        };

        let mut first_word_read = false;
        let mut common_page_key_size: u16 = 0;
        if page_tag
            .flags()
            .intersects(jet::PageTagFlags::FLAG_HAS_COMMON_KEY_SIZE)
        {
            common_page_key_size = self.clean_pgtag_flag(db_page, read_u16(self, offset)?);
            first_word_read = true;
            offset += 2;

            if common_page_key_size > 0 {
                let offset0 = page_tag_0.offset(db_page);
                let mut common_page_key =
                    self.read_bytes(offset0, common_page_key_size as usize)?;
                res.common_page_key.append(&mut common_page_key);
            }
        }

        let mut local_page_key_size: u16 = read_u16(self, offset)?;
        if !first_word_read {
            local_page_key_size = self.clean_pgtag_flag(db_page, local_page_key_size);
        }
        offset += 2;
        if local_page_key_size > 0 {
            let mut local_page_key = self.read_bytes(offset, local_page_key_size as usize)?;
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
            if common_page_key_size > 0 && local_page_key_size > 0 {
                page_key.append(&mut res.common_page_key.clone());
                page_key.append(&mut res.local_page_key.clone());
            } else if local_page_key_size > 0 {
                page_key = res.local_page_key.clone();
            } else if common_page_key_size > 0 {
                page_key = res.common_page_key.clone();
            }

            let skey: u64;
            let mut seg_offset: u32 = 0;
            // LVKEY64 (LID64, ULONG offset)
            if page_key.len() == 12 {
                skey = u64::from_le_bytes(page_key[0..8].try_into().map_err(
                    |e: TryFromSliceError| {
                        SimpleError::new(format!(
                            "can't convert page_key {:?} into slice [0..8], error: {}",
                            page_key, e
                        ))
                    },
                )?)
                .to_be();

                seg_offset = u32::from_le_bytes(
                    page_key[8..12]
                        .try_into()
                        .map_err(|e: TryFromSliceError| SimpleError::new(e.to_string()))?,
                )
                .to_be();
            } else {
                // LVKEY32 (LID32, ULONG offset)
                skey = u32::from_le_bytes(page_key[0..4].try_into().map_err(
                    |e: TryFromSliceError| {
                        SimpleError::new(format!(
                            "can't convert page_key {:?} into slice [0..4], error: {}",
                            page_key, e
                        ))
                    },
                )?)
                .to_be() as u64;

                if page_key.len() == 8 {
                    seg_offset = u32::from_le_bytes(
                        page_key[4..8]
                            .try_into()
                            .map_err(|e: TryFromSliceError| SimpleError::new(e.to_string()))?,
                    )
                    .to_be();
                }
            }

            res.offset = offset;
            res.size = (page_tag.size as u64 - (offset - page_tag_offset))
                .try_into()
                .map_err(|e: std::num::TryFromIntError| SimpleError::new(e.to_string()))?;

            let mut t: HashMap<u32, LV_tag> = HashMap::new();
            t.insert(seg_offset, res);
            let mut new_tag: LV_tags = HashMap::new();
            new_tag.insert(skey, t);

            Ok(Some(new_tag))
        }
    }

    pub fn load_lv_metadata(&self, page_number: u32) -> Result<LV_tags, SimpleError> {
        let db_page = jet::DbPage::new(self, page_number)?;

        if !db_page.flags().contains(jet::PageFlags::IS_LONG_VALUE) {
            return Err(SimpleError::new(format!(
                "pageno {}: IS_LONG_VALUE flag should be present",
                db_page.page_number
            )));
        }

        let mut tags: LV_tags = HashMap::new();

        if !db_page.flags().contains(jet::PageFlags::IS_LEAF) {
            let mut prev_page_number = page_number;
            let mut page_number =
                self.page_tag_get_branch_child_page_number(&db_page, db_page.tag(1)?)?;
            while page_number != 0 {
                let db_page = jet::DbPage::new(self, page_number)?;

                if db_page.prev_page() != 0 && prev_page_number != db_page.prev_page() {
                    return Err(SimpleError::new(format!(
                        "pageno {}: wrong previous_page number {}, expected {}",
                        db_page.page_number,
                        db_page.prev_page(),
                        prev_page_number
                    )));
                }
                if !db_page
                    .flags()
                    .contains(jet::PageFlags::IS_LEAF | jet::PageFlags::IS_LONG_VALUE)
                {
                    // maybe it's "Parent of leaf" page
                    let r = self.load_lv_metadata(page_number);
                    match r {
                        Ok(new_tags) => {
                            merge_lv_tags(&mut tags, new_tags);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    for i in 1..db_page.tags() {
                        let pg_tag = db_page.tag(i)?;
                        if jet::PageTagFlags::from_bits_truncate(pg_tag.flags)
                            .intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT)
                        {
                            continue;
                        }

                        match self.load_lv_tag(&db_page, pg_tag, db_page.tag(0)?) {
                            Ok(r) => {
                                if let Some(new_tag) = r {
                                    merge_lv_tags(&mut tags, new_tag);
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
                prev_page_number = page_number;
                page_number = db_page.next_page();
            }
        } else {
            for i in 1..db_page.tags() {
                match self.load_lv_tag(&db_page, db_page.tag(i)?, db_page.tag(0)?) {
                    Ok(r) => {
                        if let Some(new_tag) = r {
                            merge_lv_tags(&mut tags, new_tag);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(tags)
    }

    pub fn load_lv_data(
        &self,
        lv_tags: &LV_tags,
        long_value_key: u64,
        compressed: bool,
    ) -> Result<Vec<u8>, SimpleError> {
        let mut res: Vec<u8> = vec![];
        if lv_tags.contains_key(&long_value_key) {
            let seg_offsets = lv_tags.get(&long_value_key).expect("No long value key");
            loop {
                let offset = res.len() as u32;
                if seg_offsets.contains_key(&offset) {
                    let tag = seg_offsets.get(&offset).expect("No offset");
                    let mut v = self.read_bytes(tag.offset, tag.size as usize)?;
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
            Err(SimpleError::new(format!(
                "LV key 0x{:X} not found",
                long_value_key
            )))
        }
    }
}

#[derive(Debug, Clone)]
pub struct LV_tag {
    pub common_page_key: Vec<u8>,
    pub local_page_key: Vec<u8>,
    pub offset: u64,
    pub size: u32,
}

pub type LV_tags = HashMap<u64 /*key*/, HashMap<u32 /*seg_offset*/, LV_tag>>;

fn merge_lv_tags(tags: &mut LV_tags, new_tags: LV_tags) {
    for (new_key, new_segs) in new_tags {
        match tags.entry(new_key) {
            Entry::Vacant(e) => {
                e.insert(new_segs);
            }
            Entry::Occupied(mut e) => {
                let segs = e.get_mut();
                for (new_seg_offset, new_lv_tags) in new_segs {
                    segs.insert(new_seg_offset, new_lv_tags);
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_read_struct {
    ($struct_type: ident) => {
        impl $struct_type {
            pub(crate) fn read<T: ReadSeek>(
                reader: &$crate::parser::reader::Reader<T>,
                page_offset: u64,
            ) -> Result<Self, simple_error::SimpleError> {
                // eprintln!(
                //     "reads {} ({}) on {:X}",
                //     stringify!($struct_type),
                //     std::mem::size_of::<$struct_type>(),
                //     page_offset
                // );
                // crate::parser::reader::mark_used(
                //     page_offset as usize,
                //     std::mem::size_of::<$struct_type>(),
                // );
                let buffer = reader.read_bytes(page_offset, std::mem::size_of::<$struct_type>())?;
                let (_, ret) = $struct_type::parse_le(&buffer[..]).map_err(
                    |e: nom::Err<nom::error::Error<&[u8]>>| {
                        simple_error::SimpleError::new(e.to_string())
                    },
                )?;
                Ok(ret)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_read_struct_buffer {
    ($struct_type: ident) => {
        impl $struct_type {
            pub(crate) fn read<T: ReadSeek>(
                reader: &$crate::parser::reader::Reader<T>,
                page_offset: u64,
            ) -> Result<(Self, Vec<u8>), simple_error::SimpleError> {
                let buffer = reader.read_bytes(page_offset, std::mem::size_of::<$struct_type>())?;
                let (_, ret) = $struct_type::parse_le(&buffer[..]).map_err(
                    |e: nom::Err<nom::error::Error<&[u8]>>| {
                        simple_error::SimpleError::new(e.to_string())
                    },
                )?;
                Ok((ret, buffer))
            }
        }
    };
}

#[macro_export]
macro_rules! impl_read_primitive {
    ($primitive_type: ident) => {
        paste::item! {
            pub(crate) fn [<read_ $primitive_type>]<T: ReadSeek>(reader: &$crate::parser::reader::Reader<T>, page_offset: u64) -> Result<$primitive_type, simple_error::SimpleError> {
                let size = std::mem::size_of::<$primitive_type>();
                let buffer = reader.read_bytes(page_offset, size)?;
                let arr = buffer[..].try_into().map_err(|e: std::array::TryFromSliceError| simple_error::SimpleError::new(e.to_string()))?;
                Ok($primitive_type::from_le_bytes(arr))
            }
        }
    };
}
impl_read_primitive!(u8);
impl_read_primitive!(u16);
impl_read_primitive!(u32);
impl_read_primitive!(u64);

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
    pub fixed_data_bits_mask: Vec<u8>,
    pub tag_state: TaggedDataState,
    pub previous_variable_size_data_type_size: u16,
    pub var_state: VariableSizeDataState,
}

impl LastLoadState {
    pub fn init(page_number: u32, page_tag_index: usize) -> Self {
        LastLoadState {
            page_number,
            page_tag_index,
            ..Default::default()
        }
    }
}

pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl FromBytes for i8 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i8::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u8 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u8::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for i16 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i16::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u16 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u16::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for i32 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u32 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for i64 {
    fn from_bytes(bytes: &[u8]) -> Self {
        i64::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for u64 {
    fn from_bytes(bytes: &[u8]) -> Self {
        u64::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for f32 {
    fn from_bytes(bytes: &[u8]) -> Self {
        f32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromBytes for f64 {
    fn from_bytes(bytes: &[u8]) -> Self {
        f64::from_le_bytes(bytes.try_into().unwrap())
    }
}
