use crate::ese_trait::*;
pub use crate::parser::reader::*;
pub use crate::parser::*;

use simple_error::SimpleError;
use std::cell::{RefCell, RefMut};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, PartialEq)]
enum Direction {
    None,
    Forward,
    Backward,
}

// The ValidityInfo struct keeps track of pages that we have already visited
// to protect against circular reference situations
struct ValidityInfo {
    visited_pages: Vec<u32>,
    direction: Direction,
}

#[derive(Default, Debug)]
struct CurrentPage {
    current_page: Option<jet::DbPage>,
}

impl CurrentPage {
    pub(crate) fn is_none(&self) -> bool {
        self.current_page.is_none()
    }

    pub(crate) fn get(&self) -> &jet::DbPage {
        self.current_page
            .as_ref()
            .expect("Did not find current page")
    }

    pub(crate) fn set(&mut self, page: jet::DbPage) {
        self.current_page = Some(page);
    }
}

struct Table {
    cat: Box<jet::TableDefinition>,
    lv_tags: LV_tags,
    current_page: CurrentPage,
    page_tag_index: usize,
    lls: RefCell<LastLoadState>,
    validity_info: ValidityInfo,
}

impl Table {
    fn page(&self) -> &jet::DbPage {
        self.current_page.get()
    }

    fn review_last_load_state(&mut self, column: u32) {
        let mut lls = self.lls.borrow_mut();
        if lls.page_number != self.page().page_number
            || lls.page_tag_index != self.page_tag_index
            || column <= lls.last_column
        {
            // reset
            *lls = LastLoadState::init(self.page().page_number, self.page_tag_index);
        }
    }

    fn update_validity_info_for_crow(&mut self, crow: i32) {
        if crow == ESE_MoveFirst {
            self.validity_info.visited_pages.clear(); // if we're going to the beginning, clear out any previous visited into
            self.validity_info.direction = Direction::Forward
        } else if crow == ESE_MoveLast {
            self.validity_info.visited_pages.clear(); // if we're going to the end, clear out any previous visited into
            self.validity_info.direction = Direction::Backward
        }
        // We clear out the visited info if we switch direction while reading a table.
        // Otherwise if we read forward one row, then backward one row, we are reading the same row and would trigger the circular reference error.
        else if crow > 0 {
            // incrementing our row
            if self.validity_info.direction == Direction::Backward {
                self.validity_info.visited_pages.clear();
            }
            self.validity_info.direction = Direction::Forward
        } else if crow < 0 {
            // decrementing our row
            if self.validity_info.direction == Direction::Forward {
                self.validity_info.visited_pages.clear();
            }
            self.validity_info.direction = Direction::Backward
        }
    }

    fn already_visited_page(&self, page: u32) -> bool {
        self.validity_info.visited_pages.contains(&page)
    }

    fn update_visited_pages(&mut self, page: u32) {
        self.validity_info.visited_pages.push(page)
    }

    fn set_current_page(&mut self, page: jet::DbPage) -> Result<bool, SimpleError> {
        if self.already_visited_page(page.page_number) {
            Err(SimpleError::new(format!(
                "Circular page reference identified for page_number: {}",
                page.page_number
            )))
        } else {
            self.update_visited_pages(page.page_number);
            self.current_page.set(page);
            Ok(true)
        }
    }

    fn reset_visited_pages_except_current(&mut self) {
        self.validity_info.visited_pages.clear();
        self.validity_info
            .visited_pages
            .push(self.current_page.get().page_number);
    }
}

pub struct EseParser<R: ReadSeek> {
    reader: Reader<R>,
    tables: Vec<RefCell<Table>>,
}

impl EseParser<BufReader<File>> {
    /// Instantiates an instance of the parser from a file path.
    /// Does not mutate the file contents in any way.
    /// Useful for testing and sample programs.
    pub fn load_from_path(
        cache_size: usize,
        filename: impl AsRef<Path>,
    ) -> Result<Self, SimpleError> {
        let f = filename.as_ref();
        let file =
            File::open(f).unwrap_or_else(|_| panic!("File {} should exist.", f.to_string_lossy()));
        let buf_reader = BufReader::with_capacity(4096, file);
        Self::load(cache_size, buf_reader)
    }

    pub fn get_database_state(&self) -> jet::DbState {
        self.reader.db_state
    }
}

impl<R: ReadSeek> EseParser<R> {
    // reserve room for cache_size recent entries, and cache_size frequent entries
    pub fn load(cache_size: usize, read_seek: R) -> Result<Self, SimpleError> {
        let reader = Reader::load_db(read_seek, cache_size)?;
        let mut cat = reader.load_catalog()?;

        let mut tables = vec![];
        for i in cat.drain(0..) {
            if i.table_catalog_definition.is_some() {
                let itrnl = Table {
                    cat: Box::new(i),
                    lv_tags: HashMap::new(),
                    current_page: CurrentPage::default(),
                    page_tag_index: 0,
                    lls: RefCell::new(LastLoadState {
                        ..Default::default()
                    }),
                    validity_info: ValidityInfo {
                        visited_pages: vec![],
                        direction: Direction::None,
                    },
                };
                tables.push(RefCell::new(itrnl));
            }
        }

        Ok(EseParser { reader, tables })
    }

    fn get_table_by_name(
        &self,
        table: &str,
        index: &mut usize,
    ) -> Result<RefMut<Table>, SimpleError> {
        for i in 0..self.tables.len() {
            let n = self.tables[i].borrow_mut();
            if let Some(table_catalog_definition) = &n.cat.table_catalog_definition {
                if table_catalog_definition.name == table {
                    *index = i;
                    return Ok(n);
                }
            }
        }
        Err(SimpleError::new(format!("can't find table name {}", table)))
    }

    pub fn get_reader(&self) -> Result<&Reader<R>, SimpleError> {
        Ok(&self.reader)
    }

    fn get_table_by_id(&self, table_id: u64) -> Result<RefMut<Table>, SimpleError> {
        let i = table_id as usize;
        if i < self.tables.len() {
            return Ok(self.tables[i].borrow_mut());
        }
        Err(SimpleError::new(format!("out of range index {}", table_id)))
    }

    fn get_column_dyn_helper(
        &self,
        table_id: u64,
        column: u32,
        mv_index: u32,
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        let mut table = self.get_table_by_id(table_id)?;
        let reader = self.get_reader()?;
        if table.current_page.is_none() {
            return Err(SimpleError::new(
                "no current page, use open_table API before this",
            ));
        }
        table.review_last_load_state(column);
        let mut lls = table.lls.borrow_mut();
        match reader.load_data(
            &mut lls,
            &table.cat,
            &table.lv_tags,
            table.page(),
            table.page_tag_index,
            column,
            mv_index as usize,
        ) {
            Ok(r) => {
                lls.last_column = column;
                Ok(r)
            }
            Err(e) => Err(e),
        }
    }

    fn move_next_row(&self, table_id: u64, crow: i32) -> Result<bool, SimpleError> {
        let reader = self.get_reader()?;
        let mut t = self.get_table_by_id(table_id)?;
        t.update_validity_info_for_crow(crow);

        let mut i = t.page_tag_index + 1;
        if crow == ESE_MoveFirst {
            let first_leaf_page = reader.find_first_leaf_page(
                t.cat
                    .table_catalog_definition
                    .as_ref()
                    .expect("First leaf page failed")
                    .father_data_page_number,
            )?;
            if t.current_page.is_none() || t.page().page_number != first_leaf_page {
                let page = jet::DbPage::new(reader, first_leaf_page)?;
                t.set_current_page(page)?;
            } else {
                t.update_visited_pages(first_leaf_page);
            }
            if t.page().tags() < 2 {
                // empty table
                return Ok(false);
            }
            i = 1;
        }
        loop {
            while i < t.page().tags()
                && t.page()
                    .tag(i)?
                    .flags()
                    .intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT)
            {
                i += 1;
            }
            if i < t.page().tags() {
                // found non-free data tag
                t.page_tag_index = i;
                return Ok(true);
            } else if t.page().common().next_page != 0 {
                let page = jet::DbPage::new(self.get_reader()?, t.page().common().next_page)?;
                t.set_current_page(page)?;
                i = 1;
            } else {
                // no more leaf pages
                return Ok(false);
            }
        }
    }

    fn move_previous_row(&self, table_id: u64, crow: i32) -> Result<bool, SimpleError> {
        let reader = self.get_reader()?;
        let mut t = self.get_table_by_id(table_id)?;
        t.update_validity_info_for_crow(crow);

        let mut i = if t.page_tag_index > 0 {
            t.page_tag_index - 1
        } else {
            0
        };
        if crow == ESE_MoveLast {
            while t.page().common().next_page != 0 {
                let page = jet::DbPage::new(reader, t.page().common().next_page)?;
                t.set_current_page(page)?;
            }
            // in previous step we visited all pages till the end
            // now need to reset visited pages again, except last page
            t.reset_visited_pages_except_current();

            if t.page().tags() < 2 {
                // empty table
                return Ok(false);
            }
            i = t.page().tags() - 1;
        }
        loop {
            while i > 0
                && t.page()
                    .tag(i)?
                    .flags()
                    .intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT)
            {
                i -= 1;
            }
            if i > 0 {
                // found non-free data tag
                t.page_tag_index = i;
                return Ok(true);
            } else if t.page().common().previous_page != 0 {
                let page = jet::DbPage::new(reader, t.page().common().previous_page)?;
                t.set_current_page(page)?;
                i = t.page().tags() - 1;
            } else {
                // no more leaf pages
                return Ok(false);
            }
        }
    }

    fn move_row_helper(&self, table_id: u64, crow: i32) -> Result<bool, SimpleError> {
        if crow == ESE_MoveFirst || crow == ESE_MoveNext {
            self.move_next_row(table_id, crow)
        } else if crow == ESE_MoveLast || crow == ESE_MovePrevious {
            self.move_previous_row(table_id, crow)
        } else {
            match crow.cmp(&0) {
                Ordering::Greater => {
                    for _ in 0..crow {
                        if !self.move_next_row(table_id, ESE_MoveNext)? {
                            return Ok(false);
                        }
                    }
                }
                Ordering::Less => {
                    for _ in crow..0 {
                        if !self.move_previous_row(table_id, ESE_MovePrevious)? {
                            return Ok(false);
                        }
                    }
                }
                Ordering::Equal => return Ok(true),
            }
            /*
            if crow > 0 {
                for _ in 0..crow {
                    if !self.move_next_row(table_id, ESE_MoveNext)? {
                        return Ok(false);
                    }
                }
            } else if crow < 0 {
                for _ in crow..0 {
                    if !self.move_previous_row(table_id, ESE_MovePrevious)? {
                        return Ok(false);
                    }
                }
            }
            */
            Ok(true)
        }
    }

    pub fn get_fixed_column<T: FromBytes>(
        &self,
        table: u64,
        column: u32,
    ) -> Result<Option<T>, SimpleError> {
        match self.get_column(table, column)? {
            Some(v) => Ok(Some(T::from_bytes(&v))),
            None => Ok(None),
        }
    }
}

impl<R: ReadSeek> EseDb for EseParser<R> {
    fn error_to_string(&self, err: i32) -> String {
        format!("EseParser: error {}", err)
    }

    fn is_dirty(&self) -> bool {
        self.reader.is_dirty()
    }

    fn get_tables(&self) -> Result<Vec<String>, SimpleError> {
        let mut tables: Vec<String> = vec![];
        for i in &self.tables {
            let n = i.borrow();
            tables.push(
                n.cat
                    .table_catalog_definition
                    .as_ref()
                    .expect("tables are coming from table_catalog_definition")
                    .name
                    .clone(),
            );
        }
        Ok(tables)
    }

    fn open_table(&self, table: &str) -> Result<u64, SimpleError> {
        let mut index: usize = 0;
        {
            // used to drop borrow mut
            let mut t = self.get_table_by_name(table, &mut index)?;
            if let Some(long_value_catalog_definition) = &t.cat.long_value_catalog_definition {
                let reader = self.get_reader()?;
                t.lv_tags = reader
                    .load_lv_metadata(long_value_catalog_definition.father_data_page_number)?;
            }
        }
        // ignore return result
        self.move_row_helper(index as u64, ESE_MoveFirst)?;

        Ok(index as u64)
    }

    fn close_table(&self, table: u64) -> bool {
        let tags_index = table as usize;
        if tags_index < self.tables.len() {
            let mut itrnl = self.tables[tags_index].borrow_mut();
            itrnl.lv_tags.clear();
            return true;
        }
        false
    }

    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError> {
        let mut index: usize = 0;
        let t = self.get_table_by_name(table, &mut index)?;
        let mut columns: Vec<ColumnInfo> = vec![];
        for i in &t.cat.column_catalog_definition_array {
            let col_info = ColumnInfo {
                name: i.name.clone(),
                id: i.identifier,
                typ: i.column_type,
                cbmax: i.size,
                cp: i.codepage as u16,
            };
            columns.push(col_info);
        }
        Ok(columns)
    }

    fn move_row(&self, table: u64, crow: i32) -> Result<bool, SimpleError> {
        self.move_row_helper(table, crow)
            .map_err(|e| SimpleError::new(format!("move_row failed: {:?}", e)))
    }

    fn get_column(&self, table: u64, column: u32) -> Result<Option<Vec<u8>>, SimpleError> {
        self.get_column_dyn_helper(table, column, 0)
    }

    fn get_column_mv(
        &self,
        table: u64,
        column: u32,
        multi_value_index: u32,
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        self.get_column_dyn_helper(table, column, multi_value_index)
    }
}

use std::convert::TryInto;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ese_db::*;
    use std::collections::HashMap;

    fn init_table() -> Table {
        let table_definition = jet::TableDefinition {
            table_catalog_definition: None,
            column_catalog_definition_array: vec![],
            long_value_catalog_definition: None,
        };

        Table {
            cat: Box::new(table_definition),
            lv_tags: HashMap::new(),
            current_page: CurrentPage::default(),
            page_tag_index: 0,
            lls: RefCell::new(LastLoadState {
                ..Default::default()
            }),
            validity_info: ValidityInfo {
                visited_pages: vec![],
                direction: Direction::None,
            },
        }
    }

    #[test]
    fn test_validity_info_direction() {
        let mut table = init_table();

        assert_eq!(0, table.validity_info.visited_pages.len());

        // test ESE_MoveFirst
        table.update_visited_pages(10);
        assert_eq!(
            1,
            table.validity_info.visited_pages.len(),
            "Visiting page didn't increment visited_pages member"
        );
        table.update_validity_info_for_crow(ESE_MoveFirst);
        assert_eq!(
            Direction::Forward,
            table.validity_info.direction,
            "ESE_MoveFirst didn't set Direction member properly"
        );
        assert_eq!(
            0,
            table.validity_info.visited_pages.len(),
            "ESE_MoveFirst didn't clear visited_pages list"
        );

        // test continuing to move forward
        table.update_visited_pages(10);
        assert_eq!(
            1,
            table.validity_info.visited_pages.len(),
            "Visiting page didn't increment visited_pages member"
        );
        table.update_validity_info_for_crow(1);
        assert_eq!(
            Direction::Forward,
            table.validity_info.direction,
            "Moving in the same dirction switched Direction member"
        );
        assert_ne!(
            0,
            table.validity_info.visited_pages.len(),
            "Moving in the same dirction cleared out visited_pages list"
        );

        // test switching direction (forward -> backward)
        table.update_validity_info_for_crow(-1);
        assert_eq!(
            Direction::Backward,
            table.validity_info.direction,
            "Moving backward didn't switch Direction member"
        );
        assert_eq!(
            0,
            table.validity_info.visited_pages.len(),
            "Switching direction didn't clear visited_pages list"
        );

        // test switching direction (backward -> forward)
        table.update_visited_pages(10);
        assert_eq!(
            1,
            table.validity_info.visited_pages.len(),
            "Visiting page didn't increment visited_pages member"
        );
        table.update_validity_info_for_crow(1);
        assert_eq!(
            Direction::Forward,
            table.validity_info.direction,
            "Moving forward didn't switch Direction member"
        );
        assert_eq!(
            0,
            table.validity_info.visited_pages.len(),
            "Switching direction didn't clear visited_pages list"
        );

        // test ESE_MoveLast
        table.update_visited_pages(10);
        assert_eq!(
            1,
            table.validity_info.visited_pages.len(),
            "Visiting page didn't increment visited_pages member"
        );
        table.update_validity_info_for_crow(ESE_MoveLast);
        assert_eq!(
            Direction::Backward,
            table.validity_info.direction,
            "ESE_MoveLast didn't set Direction member properly"
        );
        assert_eq!(
            0,
            table.validity_info.visited_pages.len(),
            "ESE_MovESE_MoveLasteFirst didn't clear visited_pages list"
        );

        // test continuing to move backward
        table.update_visited_pages(10);
        assert_eq!(
            1,
            table.validity_info.visited_pages.len(),
            "Visiting page didn't increment visited_pages member"
        );
        table.update_validity_info_for_crow(-1);
        assert_eq!(
            Direction::Backward,
            table.validity_info.direction,
            "Moving in the same dirction switched Direction member"
        );
        assert_ne!(
            0,
            table.validity_info.visited_pages.len(),
            "Moving in the same dirction cleared out visited_pages list"
        );
    }

    #[test]
    fn test_update_visited_pages() {
        let mut table = init_table();

        assert_eq!(
            0,
            table.validity_info.visited_pages.len(),
            "Visited pages didn't start out empty"
        );
        assert!(!table.already_visited_page(15),
            "Returned true for a page we haven't visited"
        );
        table.update_visited_pages(15);
        assert_eq!(
            1,
            table.validity_info.visited_pages.len(),
            "Incorrect visited_pages len"
        );
        assert!(table.already_visited_page(15),
            "Returned false for a page we visited"
        );
        table.update_visited_pages(5);
        assert_eq!(
            2,
            table.validity_info.visited_pages.len(),
            "Incorrect visited_pages len"
        );
        assert!(table.already_visited_page(5),
            "Returned false for a page we visited"
        );
    }

    #[test]
    fn test_set_current_page() {
        let mut table = init_table();
        let page_header_old = PageHeaderOld {
            xor_checksum: 12345,
            page_number: 15,
        };
        let page_header_common = PageHeaderCommon {
            database_modification_time: jet::DateTime {
                seconds: 0,
                minutes: 0,
                hours: 0,
                day: 0,
                month: 0,
                year: 0,
                time_is_utc: 0,
                os_snapshot: 0,
            },
            previous_page: 10,
            next_page: 20,
            father_data_page_object_identifier: 0,
            available_data_size: 0,
            available_uncommitted_data_size: 0,
            available_data_offset: 0,
            available_page_tag: 0,
            page_flags: jet::PageFlags::IS_LEAF,
        };
        let page_header = PageHeader::old(page_header_old, page_header_common);

        let db_page = jet::DbPage::init_with(82, 2048, page_header, vec![]);
        assert!(table.set_current_page(db_page.clone()).unwrap(),
            "set_current_page failed for a fresh page"
        );
        assert_eq!(
            Err(SimpleError::new(
                "Circular page reference identified for page_number: 82"
            )),
            table.set_current_page(db_page),
            "set_current_page didn't error for a revisited page"
        );
    }
}
