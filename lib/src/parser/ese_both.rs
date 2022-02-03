#![cfg(target_os = "windows")]
#![allow(
    non_upper_case_globals,
    non_snake_case,
    non_camel_case_types,
    clippy::mut_from_ref,
    clippy::cast_ptr_alignment
)]

use simple_error::SimpleError;
use std::cell::RefCell;
use crate::ese_parser::*;
use crate::ese_trait::*;
use crate::esent::ese_api::*;

const CACHE_SIZE_ENTRIES: usize = 10;

pub struct EseBoth {
    api: EseAPI,
    parser: EseParser,
    opened_tables: RefCell<Vec<(u64, u64)>>,
}

impl EseBoth {
    pub fn init() -> EseBoth {
        EseBoth {
            api: EseAPI::init(),
            parser: EseParser::init(CACHE_SIZE_ENTRIES),
            opened_tables: RefCell::new(Vec::new()),
        }
    }
}

impl EseDb for EseBoth {
    fn load(&mut self, dbpath: &str) -> Option<SimpleError> {
        if let Some(e) = self.api.load(dbpath) {
            return Some(SimpleError::new(format!("EseAPI::load failed: {}", e)));
        }
        if let Some(e) = self.parser.load(dbpath) {
            return Some(SimpleError::new(format!("EseParser::load failed: {}", e)));
        }
        None
    }

    fn error_to_string(&self, _err: i32) -> String {
        "unused".to_string()
    }

    fn open_table(&self, table: &str) -> Result<u64, SimpleError> {
        let api_table = self
            .api
            .open_table(table)
            .map_err(|e| SimpleError::new(format!("EseAPI::open_table failed: {}", e)))?;
        let parser_table = self
            .parser
            .open_table(table)
            .map_err(|e| SimpleError::new(format!("EseParser::open_table failed: {}", e)))?;
        let mut v = self.opened_tables.borrow_mut();
        v.push((api_table, parser_table));
        Ok((v.len() - 1) as u64)
    }

    fn close_table(&self, table: u64) -> bool {
        let mut t = self.opened_tables.borrow_mut();
        let (api_table, parser_table) = t[table as usize];
        if !self.api.close_table(api_table) {
            println!("EseAPI::close_table({}) failed", api_table);
            return false;
        }
        if !self.parser.close_table(parser_table) {
            println!("EseParser::close_table({}) failed", parser_table);
            return false;
        }
        t.remove(table as usize);
        true
    }

    fn get_tables(&self) -> Result<Vec<String>, SimpleError> {
        let api_tables = self
            .api
            .get_tables()
            .map_err(|e| SimpleError::new(format!("EseAPI::get_tables failed: {}", e)))?;
        let parser_tables = self
            .parser
            .get_tables()
            .map_err(|e| SimpleError::new(format!("EseParser::get_tables failed: {}", e)))?;
        if api_tables.len() != parser_tables.len() {
            return Err(SimpleError::new(format!("get_tables() have a different number of tables: EseAPI tables:\n{:?}\n not equal to EseParser:\n{:?}\n",
                api_tables, parser_tables)));
        }
        for i in 0..api_tables.len() {
            if api_tables[i] != parser_tables[i] {
                return Err(SimpleError::new(format!("get_tables() have a difference: EseAPI table:\n{:?}\n not equal to EseParser:\n{:?}\n",
                    api_tables[i], parser_tables[i])));
            }
        }
        Ok(api_tables)
    }

    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError> {
        let api_columns = self
            .api
            .get_columns(table)
            .map_err(|e| SimpleError::new(format!("EseAPI::get_columns failed: {}", e)))?;
        let parser_columns = self
            .parser
            .get_columns(table)
            .map_err(|e| SimpleError::new(format!("EseParser::get_columns failed: {}", e)))?;
        if api_columns.len() != parser_columns.len() {
            if api_columns.len() > parser_columns.len()
                && (table == "MSysObjects" || table == "MSysObjectsShadow")
            {
                // there are such fields like KeyMost, LVChunkMax, SeparateLV, SpaceHints, SpaceDeferredLVHints, etc
                // useless for us (database-dependend) and not present in catalog (TODO)
                // https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc#catalog
                return Ok(parser_columns);
            }
            return Err(SimpleError::new(format!("get_columns({}) have a different number of columns: EseAPI columns:\n{:?}\n not equal to EseParser:\n{:?}\n",
                table, api_columns, parser_columns)));
        }
        for i in 0..api_columns.len() {
            if api_columns[i].id == parser_columns[i].id {
                let c1 = &api_columns[i];
                let c2 = &parser_columns[i];
                if c1.name != c2.name || c1.typ != c2.typ || c1.cbmax != c2.cbmax || c1.cp != c2.cp
                {
                    return Err(SimpleError::new(format!("get_columns({}) have a difference: EseAPI table:\n{:?}\n not equal to EseParser:\n{:?}\n",
                        table, api_columns[i], parser_columns[i])));
                }
            }
        }
        // sorted by id
        Ok(parser_columns)
    }

    fn move_row(&self, table: u64, crow: i32) -> bool {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let r1 = self.api.move_row(api_table, crow);
        let r2 = self.parser.move_row(parser_table, crow);
        if r1 != r2 {
            println!(
                "move_row return result different: EseAPI {} != EseParser {}",
                r1, r2
            );
        }
        r1
    }

    fn get_column_str(
        &self,
        table: u64,
        column: u32,
        cp: u16,
    ) -> Result<Option<String>, SimpleError> {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let s1 = self.api.get_column_str(api_table, column, cp)?;
        let s2 = self.parser.get_column_str(parser_table, column, cp)?;
        if s1 != s2 {
            return Err(SimpleError::new(format!(
                r"table {}, column({}) EseAPI column '{:?}' not equal to EseParser '{:?}'",
                table, column, s1, s2
            )));
        }
        Ok(s1)
    }

    fn get_column(&self, table: u64, column: u32) -> Result<Option<Vec<u8>>, SimpleError> {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let s1 = self.api.get_column(api_table, column)?;
        let s2 = self.parser.get_column(parser_table, column)?;
        if s1 != s2 {
            return Err(SimpleError::new(format!(
                r"table {}, column({}) EseAPI column '{:?}' not equal to EseParser '{:?}'",
                table, column, s1, s2
            )));
        }
        Ok(s1)
    }

    fn get_column_mv(
        &self,
        table: u64,
        column: u32,
        multi_value_index: u32,
    ) -> Result<Option<Vec<u8>>, SimpleError> {
        let (api_table, parser_table) = self.opened_tables.borrow()[table as usize];
        let s1 = self
            .api
            .get_column_mv(api_table, column, multi_value_index)?;
        let s2 = self
            .parser
            .get_column_mv(parser_table, column, multi_value_index)?;
        if s1 != s2 {
            return Err(SimpleError::new(format!(
                r"table {}, column({}) EseAPI column '{:?}' not equal to EseParser '{:?}'",
                table, column, s1, s2
            )));
        }
        Ok(s1)
    }
}
