use crate::vartime::*;
use byteorder::*;
use chrono::{DateTime, TimeZone, Utc};
use simple_error::SimpleError;
use std::convert::TryInto;
use std::mem;

#[derive(Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub id: u32,
    pub typ: u32,
    pub cbmax: u32,
    pub cp: u16,
}

#[derive(Debug, PartialEq)]
pub enum ESE_CP {
    None = 0,
    Unicode = 1200,
    ASCII = 1252,
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
    fn error_to_string(&self, err: i32) -> String;

    fn open_table(&self, table: &str) -> Result<u64, SimpleError>;
    fn close_table(&self, table: u64) -> bool;

    fn get_tables(&self) -> Result<Vec<String>, SimpleError>;
    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError>;

    fn get_column(&self, table: u64, column: u32) -> Result<Option<Vec<u8>>, SimpleError>;
    fn get_column_mv(
        &self,
        table: u64,
        column: u32,
        multi_value_index: u32,
    ) -> Result<Option<Vec<u8>>, SimpleError>;

    fn move_row(&self, table: u64, crow: i32) -> Result<bool, SimpleError>;

    fn get_column_date(
        &self,
        table: u64,
        column: u32,
    ) -> Result<Option<DateTime<Utc>>, SimpleError> {
        let r = self.get_column(table, column)?;
        if let Some(v) = r {
            let vartime = f64::from_le_bytes(v.clone().try_into().unwrap());
            let mut st = SYSTEMTIME::default();
            if VariantTimeToSystemTime(vartime as f64, &mut st) {
                // this is obviously not the right function! I didn't know what the right one was off the top of my head.
                // We need to include the time component. also needs to be something that returns a DateTime.
                let datetime = Utc.with_ymd_and_hms(
                    st.wYear as i32,
                    st.wMonth as u32,
                    st.wDay as u32,
                    st.wHour as u32,
                    st.wMinute as u32,
                    st.wSecond as u32,
                );
                Ok(datetime.single())
            } else {
                let filetime = u64::from_le_bytes(v.try_into().unwrap());
                let datetime = get_date_time_from_filetime(filetime);
                Ok(Some(datetime))
            }
        } else {
            Ok(None)
        }
    }

    fn get_column_str(
        &self,
        table: u64,
        column: u32,
        cp: u16,
    ) -> Result<Option<String>, SimpleError> {
        use std::convert::TryFrom;
        let r = self.get_column(table, column)?;
        if let Some(v) = r {
            if ESE_CP::try_from(cp).expect("Failed to get ESE cp") == ESE_CP::Unicode {
                let mut vec16: Vec<u16> = vec![0; v.len() / mem::size_of::<u16>()];
                LittleEndian::read_u16_into(&v, &mut vec16);
                match String::from_utf16(&vec16[..]) {
                    Ok(s) => return Ok(Some(s)),
                    Err(e) => {
                        return Err(SimpleError::new(format!(
                            "String::from_utf16 failed: {}",
                            e
                        )))
                    }
                }
            } else {
                match std::str::from_utf8(&v) {
                    Ok(s) => return Ok(Some(s.to_string())),
                    Err(e) => {
                        return Err(SimpleError::new(format!(
                            "std::str::from_utf8 failed: {}",
                            e
                        )))
                    }
                }
            }
        } else {
            Ok(None)
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
