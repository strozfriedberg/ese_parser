
use simple_error::SimpleError;

#[derive(Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub id: u32,
    pub typ: u32,
    pub cbmax: u32,
    pub cp: u16
}

pub trait EseDb {
    fn init() -> Self;
    fn load(&mut self, dbpath: &str, cache_size: usize) -> Option<SimpleError>;

    fn error_to_string(&self, err: i32) -> String;

    fn open_table(&self, table: &str) -> Result<u64, SimpleError>;
    fn close_table(&self, table: u64) -> bool;

    fn get_tables(&self) -> Result<Vec<String>, SimpleError>;
    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError>;

    fn get_column_str(&self, table: u64, column: u32, size: u32)
        -> Result<Option<String>, SimpleError>;
    fn get_column<T>(&self, table: u64, column: u32) -> Result<Option<T>, SimpleError>;
    fn get_column_dyn(&self, table: u64, column: u32, size: usize)
        -> Result< Option<Vec<u8>>, SimpleError>;
    fn get_column_dyn_varlen(&self, table: u64, column: u32)
        -> Result< Option<Vec<u8>>, SimpleError>;

    fn move_row(&self, table: u64, crow: u32) -> bool;
}
