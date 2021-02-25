
use crate::parser::*;
use crate::ese_trait::*;
use crate::parser::reader::*;
use crate::esent;

use std::convert::TryFrom;
use simple_error::SimpleError;
use std::cell::{RefCell, Ref, RefMut};

struct Internal {
    cat: Box<jet::TableDefinition>,
    lv_tags: Vec<LV_tags>,
    current_page: Option<jet::DbPage>,
    page_tag_index: usize
}

pub struct EseParser {
    io_handle: Option<jet::IoHandle>,
    tables: Vec<RefCell<Internal>>,
}

impl Internal {
    fn page(&self) -> &jet::DbPage {
        self.current_page.as_ref().unwrap()
    }
}

impl EseParser {
    fn get_table(&self, table: &str, index: &mut usize) -> Result<RefMut<Internal>, SimpleError> {
        for i in 0..self.tables.len() {
            let n = self.tables[i].borrow_mut();
            if n.cat.table_catalog_definition.as_ref().unwrap().name == table {
                *index = i;
                return Ok(n);
            }
        }
        Err(SimpleError::new(format!("can't find table name {}", table)))
    }

    fn get_handle(&self) -> Result<&jet::IoHandle, SimpleError> {
        match &self.io_handle {
            Some(h) => Ok(&h),
            None => Err(SimpleError::new("IoHandle is uninit, database opened?"))
        }
    }

    fn get_internal(&self, table: u64) -> Result<RefMut<Internal>, SimpleError> {
        let i = table as usize;
        if i < self.tables.len() {
            return Ok(self.tables[i].borrow_mut());
        }
        Err(SimpleError::new(format!("out of range index {}", table)))
    }

    fn get_column_dyn_helper(&self, table: u64, column: u32) -> Result<Option<Vec<u8>>, SimpleError> {
        let itrnl = self.get_internal(table)?;
        let io_handle = self.get_handle()?;
        if itrnl.current_page.is_none() {
            return Err(SimpleError::new("no current page, use open_table API before this"));
        }
        load_data(io_handle, &itrnl.cat, &itrnl.lv_tags, &itrnl.page(), itrnl.page_tag_index, column)
    }

    fn move_row_helper(&self, table: u64, crow: u32) -> Result<bool, SimpleError> {
        let io_handle = self.get_handle()?;
        let mut t = self.get_internal(table)?;

        if crow == esent::JET_MoveFirst as u32 || crow == esent::JET_MoveNext as u32 {
            let mut i = t.page_tag_index + 1;
            if crow == esent::JET_MoveFirst as u32 {
                let first_leaf_page = find_first_leaf_page(io_handle,
                    t.cat.table_catalog_definition.as_ref().unwrap().father_data_page_number)?;
                if t.current_page.is_none() || t.page().page_number != first_leaf_page {
                    let page = jet::DbPage::new(&self.io_handle.as_ref().unwrap(), first_leaf_page)?;
                    t.current_page = Some(page);
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
                    (*t).page_tag_index = i;
                    return Ok(true);
                } else {
                    if t.page().common().next_page != 0 {
                        let page = jet::DbPage::new(&self.io_handle.as_ref().unwrap(), t.page().common().next_page)?;
                        t.current_page = Some(page);
                        i = 1;
                    } else {
                        // no more leaf pages
                        return Ok(false);
                    }
                }
            }
        } else if crow == esent::JET_MoveLast as u32 || crow == esent::JET_MovePrevious as u32 {
            let mut i = t.page_tag_index - 1;
            if crow == esent::JET_MoveLast as u32 {
                while t.page().common().next_page != 0 {
                    let page = jet::DbPage::new(&self.io_handle.as_ref().unwrap(), t.page().common().next_page)?;
                    t.current_page = Some(page);
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
                } else {
                    if t.page().common().previous_page != 0 {
                        let page = jet::DbPage::new(&self.io_handle.as_ref().unwrap(), t.page().common().previous_page)?;
                        t.current_page = Some(page);
                        i = t.page().page_tags.len()-1;
                    } else {
                        // no more leaf pages
                        return Ok(false);
                    }
                }
            }
        } else {
            // movo to crow
        }

        Err(SimpleError::new(format!("move_row: TODO: implement me, crow {}", crow)))
    }
}

impl EseDb for EseParser {
    fn init() -> EseParser {
        EseParser { io_handle: None, tables: vec![] }
    }

    fn load(&mut self, dbpath: &str) -> Option<SimpleError> {
        let h = match jet::IoHandle::load_db(&std::path::PathBuf::from(dbpath)) {
            Ok(h) => h,
            Err(e) => {
                return Some(SimpleError::new(e.to_string()));
            }
        };
        let mut cat = match load_catalog(&h) {
            Ok(c) => c,
            Err(e) => return Some(e)
        };
        self.io_handle = Some(h);
        for i in cat.drain(0..) {
            if i.table_catalog_definition.is_some() {
                let itrnl = Internal { cat: Box::new(i), lv_tags: vec![], current_page: None, page_tag_index: 0 };
                self.tables.push(RefCell::new(itrnl));
            }
        }
        None
    }

    fn error_to_string(&self, err: i32) -> String {
        "TODO".to_string()
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
            let mut t = self.get_table(table, &mut index)?;
            let io_handle = self.get_handle()?;
            let mut lv : Vec<LV_tags> = Vec::new();
            if t.cat.long_value_catalog_definition.is_some() {
                lv = load_lv_metadata(io_handle,
                    t.cat.long_value_catalog_definition.as_ref().unwrap().father_data_page_number)?;
                t.lv_tags = lv;
            }
        }
        // ignore return result
        self.move_row_helper(index as u64, esent::JET_MoveFirst);

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
        let t = self.get_table(table, &mut index)?;
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

    fn move_row(&self, table: u64, crow: u32) -> bool {
        match self.move_row_helper(table, crow) {
            Ok(r) => r,
            Err(e) => {
                println!("move_row_helper failed: {:?}", e);
                return false;
            }
        }
    }

    fn get_column_str(&self, table: u64, column: u32, size: u32) -> Result<Option<String>, SimpleError> {
        let v = self.get_column_dyn_helper(table, column)?;
        if v.is_none() {
            return Ok(None);
        }
        match std::str::from_utf8(&v.unwrap()) {
            Ok(s) => Ok(Some(s.to_string())),
            Err(e) => Err(SimpleError::new(format!("std::str::from_utf8 failed: {}", e)))
        }
    }

    fn get_column<T>(&self, table: u64, column: u32) -> Result<Option<T>, SimpleError> {
        let size = std::mem::size_of::<T>();
        let mut dst = std::mem::MaybeUninit::<T>::zeroed();

        unsafe {
            let vo = self.get_column_dyn_helper(table, column)?;
            if vo.is_none() {
                return Err(SimpleError::new(format!("get_column_dyn_helper: 0 size returned, expected {}", size)));
            }
            let v = vo.as_ref().unwrap();
            if size != v.len() {
                return Err(SimpleError::new(format!("get_column_dyn_helper: wrong size ({}) returned, expected {}",
                    v.len(), size)));
            }
            std::ptr::copy_nonoverlapping(
                v.as_ptr(),
                dst.as_mut_ptr() as *mut u8,
                size);
            Ok(Some(dst.assume_init()))
        }
    }

    fn get_column_dyn(&self, table: u64, column: u32, size: usize) -> Result< Option<Vec<u8>>, SimpleError> {
        self.get_column_dyn_helper(table, column)
    }

    fn get_column_dyn_varlen(&self, table: u64, column: u32) -> Result< Option<Vec<u8>>, SimpleError> {
        self.get_column_dyn_helper(table, column)
    }
}
