use simple_error::SimpleError;

#[derive(Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub id: u32,
    pub typ: u32,
    pub cbmax: u32,
    pub cp: u16
}

#[derive(Debug, PartialEq)]
pub enum ESE_CP {
    None = 0,
    Unicode = 1200,
    ASCII = 1252
}

impl std::convert::TryFrom<u16> for ESE_CP {
    type Error = ();

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            x if x == ESE_CP::None as u16 => Ok(ESE_CP::None),
            x if x == ESE_CP::ASCII as u16 => Ok(ESE_CP::ASCII),
            x if x == ESE_CP::Unicode as u16 => Ok(ESE_CP::Unicode),
            _ => Err(()),
        }
    }
}

pub const ESE_coltypBit: u32 = 1;
pub const ESE_coltypUnsignedByte: u32 = 2;
pub const ESE_coltypShort: u32 = 3;
pub const ESE_coltypLong: u32 = 4;
pub const ESE_coltypCurrency: u32 = 5;
pub const ESE_coltypIEEESingle: u32 = 6;
pub const ESE_coltypIEEEDouble: u32 = 7;
pub const ESE_coltypDateTime: u32 = 8;
pub const ESE_coltypBinary: u32 = 9;
pub const ESE_coltypText: u32 = 10;
pub const ESE_coltypLongBinary: u32 = 11;
pub const ESE_coltypLongText: u32 = 12;
pub const ESE_coltypSLV: u32 = 13;
pub const ESE_coltypUnsignedLong: u32 = 14;
pub const ESE_coltypLongLong: u32 = 15;
pub const ESE_coltypGUID: u32 = 16;
pub const ESE_coltypUnsignedShort: u32 = 17;
pub const ESE_coltypUnsignedLongLong: u32 = 18;
pub const ESE_coltypMax: u32 = 19;

pub const ESE_MoveFirst: i32 = -2147483648;
pub const ESE_MovePrevious: i32 = -1;
pub const ESE_MoveNext: i32 = 1;
pub const ESE_MoveLast: i32 = 2147483647;

pub trait EseDb {
    fn load(&mut self, dbpath: &str) -> Option<SimpleError>;

    fn error_to_string(&self, err: i32) -> String;

    fn open_table(&self, table: &str) -> Result<u64, SimpleError>;
    fn close_table(&self, table: u64) -> bool;

    fn get_tables(&self) -> Result<Vec<String>, SimpleError>;
    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError>;

    fn get_column(&self, table: u64, column: u32) -> Result< Option<Vec<u8>>, SimpleError>;
    fn get_column_mv(&self, table: u64, column: u32, multi_value_index: u32) -> Result< Option<Vec<u8>>, SimpleError>;

    fn move_row(&self, table: u64, crow: i32) -> bool;

	fn get_column_str(&self, table: u64, column: u32, cp: u16) -> Result<Option<String>, SimpleError> {
		use std::convert::TryFrom;
		let r = self.get_column(table, column)?;
		if let Some(v) = r {
			if ESE_CP::try_from(cp).expect("Failed to get ESE cp") == ESE_CP::Unicode {
				let buf = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u16, v.len() / 2) };
				match String::from_utf16(buf) {
					Ok(s) => return Ok(Some(s)),
					Err(e) => return Err(SimpleError::new(format!("String::from_utf16 failed: {}", e)))
				}
			} else {
				match std::str::from_utf8(&v) {
					Ok(s) => return Ok(Some(s.to_string())),
					Err(e) => return Err(SimpleError::new(format!("std::str::from_utf8 failed: {}", e)))
				}
			}
		}
		Err(SimpleError::new(format!("can't decode string from {:?}", r)))
	}
}
