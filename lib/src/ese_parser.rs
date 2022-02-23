use crate::parser::*;
use crate::ese_trait::*;
use crate::parser::reader::*;

use simple_error::SimpleError;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::cmp::Ordering;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

#[derive(PartialEq)]
enum Direction {
    None,
    Forward,
    Backward
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
        self.current_page.as_ref().expect("Did not find current page")
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
    validity_info: ValidityInfo
}

impl Table {
    fn page(&self) -> &jet::DbPage {
        self.current_page.get()
    }

	fn review_last_load_state(&mut self, column: u32) {
		let mut lls = self.lls.borrow_mut();
		if lls.page_number != self.page().page_number || lls.page_tag_index != self.page_tag_index ||
            column <= lls.last_column {
			// reset
			*lls = LastLoadState::init(self.page().page_number, self.page_tag_index);
		}
	}

    fn clear_validity_lists(&mut self) {
        self.validity_info.visited_pages.clear();
    }

    fn update_validity_info(&mut self, crow: i32) {
        if crow == ESE_MoveFirst {
            self.clear_validity_lists(); // if we're going to the beginning, clear out any previous visited into
            self.validity_info.direction = Direction::Forward
        }
        else if crow == ESE_MoveLast {
            self.clear_validity_lists(); // if we're going to the end, clear out any previous visited into
            self.validity_info.direction = Direction::Backward
        }
        // We clear out the visited info if we switch direction while reading a table.
        // Otherwise if we read forward one row, then backward one row, we are reading the same row and would trigger the circular reference error.
        else if crow > 0 { // incrementing our row
            if self.validity_info.direction == Direction::Backward {
                self.clear_validity_lists();
            }
            self.validity_info.direction = Direction::Forward
        }
        else if crow < 0 { // decrementing our row
            if self.validity_info.direction == Direction::Forward {
                self.clear_validity_lists();
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
        }
        else {
            self.update_visited_pages(page.page_number);
            self.current_page.set(page);
            Ok(true)
        }
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
    pub fn load_from_path(cache_size: usize, filename: impl AsRef<Path>) -> Result<Self, SimpleError> {
        let f = filename.as_ref();
        let file = File::open(f).unwrap();
        let buf_reader = BufReader::with_capacity(4096, file);

        Self::load(cache_size, buf_reader)
    }
}

impl<R: ReadSeek> EseParser<R> {
     // reserve room for cache_size recent entries, and cache_size frequent entries
    pub fn load(cache_size: usize, read_seek: R) -> Result<Self, SimpleError> {
        let reader =  Reader::load_db(read_seek, cache_size)?;
        let mut cat = reader.load_catalog()?;

        let mut tables =  vec![];
        for i in cat.drain(0..) {
            if i.table_catalog_definition.is_some() {
                let itrnl =
                    Table {
                        cat: Box::new(i),
                        lv_tags: HashMap::new(),
                        current_page: CurrentPage::default(),
                        page_tag_index: 0,
                        lls: RefCell::new( LastLoadState { ..Default::default() }),
                        validity_info: ValidityInfo {
                            visited_pages: vec![],
                            direction: Direction::None
                        }
                    };
                tables.push(RefCell::new(itrnl));
            }
        }

        Ok(
            EseParser {
                reader,
                tables
            }
        )
    }

    fn get_table_by_name(&self, table: &str, index: &mut usize) -> Result<RefMut<Table>, SimpleError> {
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

    fn get_reader(&self) -> Result<&Reader<R>, SimpleError> {
        Ok(&self.reader)
    }

    fn get_table_by_id(&self, table_id: u64) -> Result<RefMut<Table>, SimpleError> {
        let i = table_id as usize;
        if i < self.tables.len() {
            return Ok(self.tables[i].borrow_mut());
        }
        Err(SimpleError::new(format!("out of range index {}", table_id)))
    }

    fn get_column_dyn_helper(&self, table_id: u64, column: u32, mv_index: u32) -> Result<Option<Vec<u8>>, SimpleError> {
        let mut table = self.get_table_by_id(table_id)?;
        let reader = self.get_reader()?;
        if table.current_page.is_none() {
            return Err(SimpleError::new("no current page, use open_table API before this"));
        }
		table.review_last_load_state(column);
		let mut lls = table.lls.borrow_mut();
        match reader.load_data(&mut lls, &table.cat, &table.lv_tags, table.page(), table.page_tag_index, column,
			mv_index as usize) {
			Ok(r) => {
				lls.last_column = column;
				Ok(r)
			},
			Err(e) => Err(e)
		}
    }

    fn move_next_row(&self, table_id: u64, crow: i32) -> Result<bool, SimpleError> {
        let reader = self.get_reader()?;
        let mut t = self.get_table_by_id(table_id)?;
        t.update_validity_info(crow);

        let mut i = t.page_tag_index + 1;
        if crow == ESE_MoveFirst {
            let first_leaf_page = reader.find_first_leaf_page(
                t.cat.table_catalog_definition.as_ref().expect("First leaf page failed").father_data_page_number)?;
            if t.current_page.is_none() || t.page().page_number != first_leaf_page {
                let page = jet::DbPage::new(reader, first_leaf_page)?;
                t.set_current_page(page)?;
            }
            else {
                t.update_visited_pages(first_leaf_page);
            }
            if t.page().page_tags.len() < 2 {
                // empty table
                return Ok(false);
            }
            i = 1;
        }
        loop {
            while i < t.page().page_tags.len() &&
                t.page().page_tags[i].flags().intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT) {
                i += 1;
            }
            if i < t.page().page_tags.len() {
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
        t.update_validity_info(crow);

        let mut i = t.page_tag_index - 1;
        if crow == ESE_MoveLast {
            while t.page().common().next_page != 0 {
                let page = jet::DbPage::new(reader, t.page().common().next_page)?;
                t.set_current_page(page)?;
            }
            if t.page().page_tags.len() < 2 {
                // empty table
                return Ok(false);
            }
            i = t.page().page_tags.len()-1;
        }
        loop {
            while i > 0 && t.page().page_tags[i].flags().intersects(jet::PageTagFlags::FLAG_IS_DEFUNCT) {
                i -= 1;
            }
            if i > 0 {
                // found non-free data tag
                t.page_tag_index = i;
                return Ok(true);
            } else if t.page().common().previous_page != 0 {
                    let page = jet::DbPage::new(reader, t.page().common().previous_page)?;
                    t.set_current_page(page)?;
                    i = t.page().page_tags.len()-1;
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
                },
                Ordering::Less => {
                    for _ in crow..0 {
                        if !self.move_previous_row(table_id, ESE_MovePrevious)? {
                            return Ok(false);
                        }
                    }
				},
                Ordering::Equal => {
                    return Ok(true)
                },
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

    pub fn get_fixed_column<T: FromBytes>(&self, table: u64, column: u32) -> Result<Option<T>, SimpleError> {
        match self.get_column(table, column)? {
            Some(v) => Ok(Some(T::from_bytes(&v))),
            None => Ok(None)
        }
    }
}

impl<R: ReadSeek> EseDb for EseParser<R> {
    fn error_to_string(&self, err: i32) -> String {
        format!("EseParser: error {}", err)
    }

    fn get_tables(&self) -> Result<Vec<String>, SimpleError> {
        let mut tables : Vec<String> = vec![];
        for i in &self.tables {
            let n = i.borrow();
            tables.push(n.cat.table_catalog_definition.as_ref().unwrap().name.clone());
        }
        Ok(tables)
    }

    fn open_table(&self, table: &str) -> Result<u64, SimpleError> {
        let mut index : usize = 0;
        { // used to drop borrow mut
            let mut t = self.get_table_by_name(table, &mut index)?;
            if let Some(long_value_catalog_definition) = &t.cat.long_value_catalog_definition {
                let reader = self.get_reader()?;
                t.lv_tags = reader.load_lv_metadata(long_value_catalog_definition.father_data_page_number)?;
            }

            // if t.cat.long_value_catalog_definition.is_some() {
			// 	let reader = self.get_reader()?;
            //     t.lv_tags = load_lv_metadata(reader,
            //         t.cat.long_value_catalog_definition.as_ref().map_err(|e: std::num::TryFromIntError| SimpleError::new(e.to_string())).father_data_page_number)?;
            // }
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
        let mut index : usize = 0;
        let t = self.get_table_by_name(table, &mut index)?;
        let mut columns : Vec<ColumnInfo> = vec![];
        for i in &t.cat.column_catalog_definition_array {
            let col_info = ColumnInfo {
                  name: i.name.clone(),
                    id: i.identifier,
                   typ: i.column_type,
                 cbmax: i.size,
                    cp: i.codepage as u16
            };
            columns.push(col_info);
        }
        Ok(columns)
    }

    fn move_row(&self, table: u64, crow: i32) -> bool {
        match self.move_row_helper(table, crow) {
            Ok(r) => r,
            Err(e) => {
                println!("move_row_helper failed: {:?}", e);
                false
            }
        }
    }

    fn get_column(&self, table: u64, column: u32) -> Result< Option<Vec<u8>>, SimpleError> {
        self.get_column_dyn_helper(table, column, 0)
    }

    fn get_column_mv(&self, table: u64, column: u32, multi_value_index: u32)
        -> Result< Option<Vec<u8>>, SimpleError> {
        self.get_column_dyn_helper(table, column, multi_value_index)
    }
}

use std::convert::TryInto;

pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl FromBytes for i8 {
    fn from_bytes(bytes: &[u8]) -> Self  { i8::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for u8 {
    fn from_bytes(bytes: &[u8]) -> Self  { u8::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for i16 {
    fn from_bytes(bytes: &[u8]) -> Self  { i16::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for u16 {
    fn from_bytes(bytes: &[u8]) -> Self  { u16::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for i32 {
    fn from_bytes(bytes: &[u8]) -> Self  { i32::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for u32 {
    fn from_bytes(bytes: &[u8]) -> Self  { u32::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for i64 {
    fn from_bytes(bytes: &[u8]) -> Self  { i64::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for u64 {
    fn from_bytes(bytes: &[u8]) -> Self  { u64::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for f32 {
    fn from_bytes(bytes: &[u8]) -> Self  { f32::from_le_bytes(bytes.try_into().unwrap()) }
}

impl FromBytes for f64 {
    fn from_bytes(bytes: &[u8]) -> Self  { f64::from_le_bytes(bytes.try_into().unwrap()) }
}