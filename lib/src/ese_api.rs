use crate::ese_trait::*;
use crate::esent::*;

use simple_error::SimpleError;

use std::ffi::CString;
use std::os::raw::{c_void, c_ulong};
use std::mem::{size_of, MaybeUninit};

#[derive(Debug)]
pub struct EseAPI {
    instance: JET_INSTANCE,
    sesid: JET_SESID,
    dbid: JET_DBID,
}

impl EseAPI {
    fn get_column_info(&self, table: &str, column: &str) -> Result<JET_COLUMNBASE_A, SimpleError> {
        let tbl = CString::new(table).unwrap();
        let col = CString::new(column).unwrap();
        let mut col_base = MaybeUninit::<JET_COLUMNBASE_A>::zeroed();
        unsafe {
            let res = JetGetColumnInfoA(self.sesid, self.dbid, tbl.as_ptr(),
                col.as_ptr(), col_base.as_mut_ptr() as *mut c_void, size_of::<JET_COLUMNBASE_A>() as c_ulong, JET_ColInfoBase);
            if res != 0 {
                return Err(SimpleError::new(
                    format!("JetOpenDatabaseA failed with error {}", self.error_to_string(res))));
            }
            Ok(col_base.assume_init())
        }
    }
}

impl EseDb for EseAPI {

    fn init() -> EseAPI {
        EseAPI { instance: 0, sesid: 0, dbid: 0 }
    }

    fn load(&mut self, dbpath: &str) -> Option<SimpleError> {
        let dbinfo = get_database_file_info(dbpath).unwrap();
        set_system_parameter_l(JET_paramDatabasePageSize, (dbinfo.cbPageSize as u64).into());
        set_system_parameter_l(JET_paramDisableCallbacks, (true as u64).into());
        set_system_parameter_s(JET_paramRecovery, "Off");

        let mut instance : JET_INSTANCE = 0;
        unsafe {
            let res = JetCreateInstanceA(&mut instance, std::ptr::null());
            if res != 0 {
                return Some(SimpleError::new(format!("JetCreateInstanceA failed with error: {}", res)));
            }
        }
        unsafe {
            let res = JetInit(&mut instance);
            if res != 0 {
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetInit failed with error {}", res)));
            }
        }

        let mut sesid : JET_SESID = 0;
        unsafe {
            let res = JetBeginSessionA(instance, &mut sesid, std::ptr::null(), std::ptr::null() );
            if res != 0 {
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetBeginSessionA failed with error {}", res)));
            }
        }

        let dbpath = CString::new(dbpath).unwrap();
        unsafe {
            let res = JetAttachDatabaseA(sesid, dbpath.as_ptr(), JET_bitDbReadOnly);
            if res != 0 {
                JetEndSession(sesid, 0);
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetAttachDatabaseA failed with error {}", res)));
            }
        }

        let mut dbid : JET_DBID = 0;
        unsafe {
            let res = JetOpenDatabaseA(sesid, dbpath.as_ptr(), std::ptr::null(), &mut dbid, JET_bitDbReadOnly);
            if res != 0 {
                JetDetachDatabaseA(sesid, std::ptr::null());
                JetEndSession(sesid, 0);
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetOpenDatabaseA failed with error {}", res)));
            }
        }

        self.instance = instance;
        self.sesid = sesid;
        self.dbid = dbid;
        None
    }

    fn error_to_string(&self, err: JET_ERR) -> String {
        let mut v : Vec<u8> = Vec::new();
        v.resize(256, 0);
        unsafe {
            let res = JetGetSystemParameterA(self.instance, self.sesid, JET_paramErrorToString,
                (err as *mut u64).into(), v.as_mut_ptr() as *mut i8, v.len() as c_ulong);
            if res != 0 {
                std::str::from_utf8(&v).unwrap().to_string()
            } else {
                format!("JetGetSystemParameterA failed with error {}", res)
            }
        }
    }

    fn open_table(&self, table: &str) -> Result<JET_TABLEID, SimpleError> {
        let tbl = CString::new(table).unwrap();
        let mut tableid : JET_TABLEID = 0;
        unsafe {
            let res = JetOpenTableA(self.sesid, self.dbid, tbl.as_ptr(), std::ptr::null(), 0, JET_bitTableReadOnly,
                &mut tableid);
            if res != 0 {
                return Err(SimpleError::new(format!("JetOpenTableA failed with error {}", self.error_to_string(res))));
            }
            Ok(tableid)
        }
    }

    fn close_table(&self, table: JET_TABLEID) -> bool {
        unsafe {
            let res = JetCloseTable(self.sesid, table);
            res != 0
        }
    }

    fn get_column_str(&self, table: JET_TABLEID, column: JET_COLUMNID, size: u32) -> Result<Option<String>, SimpleError> {
        let mut bytes : c_ulong = 0;
        let mut v : Vec<u8> = Vec::new();
        v.resize(size as usize, 0);

        unsafe {
            let res = JetRetrieveColumn(self.sesid, table, column, v.as_mut_ptr() as *mut c_void, size,
                &mut bytes, 0, std::ptr::null_mut());
            if res != 0 {
                if res == JET_wrnColumnNull as i32 {
                    return Ok(None);
                }
                return Err(SimpleError::new(
                    format!("JetRetrieveColumn failed with error {}", self.error_to_string(res))));
            }
        }
        v.truncate(bytes as usize);

        match std::str::from_utf8(&v) {
            Ok(s) => Ok(Some(s.to_string())),
            Err(e) => Err(SimpleError::new(format!("std::str::from_utf8 failed: {}", e)))
        }
    }

    fn get_column<T>(&self, table: JET_TABLEID, column: JET_COLUMNID) -> Result<Option<T>, SimpleError> {
        let mut bytes : c_ulong = 0;
        let size : c_ulong = size_of::<T>() as u32;
        let mut v = MaybeUninit::<T>::zeroed();

        unsafe {
            let res = JetRetrieveColumn(self.sesid, table, column, v.as_mut_ptr() as *mut c_void, size,
                &mut bytes, 0, std::ptr::null_mut());
            if res != 0 {
                if res == JET_wrnColumnNull as i32 {
                    return Ok(None);
                }
                return Err(SimpleError::new(
                    format!("JetRetrieveColumn failed with error {}", self.error_to_string(res))));
            }

            Ok(Some(v.assume_init()))
        }
    }

    fn get_column_dyn2(&self, table: JET_TABLEID, column: JET_COLUMNID, data: &mut[u8], size: usize)
        -> Result<u32, SimpleError> {
        let mut bytes : c_ulong = 0;
        unsafe {
            let res = JetRetrieveColumn(self.sesid, table, column, data.as_mut_ptr() as *mut c_void, size as u32,
                &mut bytes, 0, std::ptr::null_mut());
            if res != 0 {
                if res == JET_wrnColumnNull as i32 {
                    return Ok(0);
                }
                return Err(SimpleError::new(
                    format!("JetRetrieveColumn failed with error {}", self.error_to_string(res))));
            }

            Ok(bytes as u32)
        }
    }

    fn get_column_dyn(&self, table: JET_TABLEID, column: JET_COLUMNID, size: usize) -> Result< Option<Vec<u8>>, SimpleError> {
        let mut v : Vec<u8> = Vec::new();
        v.resize(size, 0);
        match self.get_column_dyn2(table, column, v.as_mut_slice(), size) {
            Err(e) => Err(e),
            Ok(s) => {
                if s == 0{
                    return Ok(None);
                }
                if s > size as u32 {
                    return Err(SimpleError::new(format!("wrong size {}, expected {}", s, size)));
                }
                v.truncate(s as usize);
                Ok(Some(v))
            }
        }
    }

    fn get_column_dyn_varlen(&self, table: JET_TABLEID, column: JET_COLUMNID) -> Result< Option<Vec<u8>>, SimpleError> {
        let mut vres : Vec<u8> = Vec::new();

        loop {
            let mut v : Vec<u8> = Vec::new();
            let size: usize = 4096;
            v.resize(size, 0);

            let mut bytes : c_ulong = 0;
            unsafe {
                let mut retinfo = JET_RETINFO { cbStruct: size_of::<JET_RETINFO>() as c_ulong, ibLongValue: vres.len() as c_ulong, itagSequence : 1, columnidNextTagged: 0 };
                let res = JetRetrieveColumn(self.sesid, table, column, v.as_mut_slice().as_mut_ptr() as *mut c_void, size as u32,
                    &mut bytes, 0, &mut retinfo);
                if res != 0 && res != JET_wrnBufferTruncated as i32 {
                    if res == JET_wrnColumnNull as i32 {
                        return Ok(None);
                    }
                    return Err(SimpleError::new(
                        format!("JetRetrieveColumn failed with error {}", self.error_to_string(res))));
                }

                v.truncate(bytes as usize);
                vres.append(v.as_mut());
                if res != JET_wrnBufferTruncated as i32 {
                    break;
                }
            }
        }
        Ok(Some(vres))
    }

    fn move_row(&self, table: JET_TABLEID, crow: u32) -> bool {
        unsafe {
            let res = JetMove(self.sesid, table, crow as std::os::raw::c_long, 0);
            res == 0
        }
    }

    fn get_tables(&self) -> Result<Vec<String>, SimpleError> {
        let c_name_info = self.get_column_info("MSysObjects", "Name")?;
        let c_type_info = self.get_column_info("MSysObjects", "Type")?;

        let table_id = self.open_table("MSysObjects")?;

        let mut res : Vec<String> = Vec::new();
        loop {
            let name_str = self.get_column_str(table_id, c_name_info.columnid, c_name_info.cbMax)?.unwrap();
            let type_word = self.get_column::<u16>(table_id, c_type_info.columnid)?.unwrap();

            if type_word == 1 {
               res.push(name_str);
            }

            if !self.move_row(table_id, JET_MoveNext) {
                break;
            }
        }

        self.close_table(table_id);
        Ok(res)
    }

    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError>
    {
        let table_id = self.open_table(table)?;
        let mut cols : Vec<ColumnInfo> = Vec::new();
        let mut col_list = MaybeUninit::<JET_COLUMNLIST>::zeroed();
        unsafe {
            let res = JetGetTableColumnInfoA(self.sesid, table_id, std::ptr::null(),
                col_list.as_mut_ptr() as *mut c_void, size_of::<JET_COLUMNLIST>() as c_ulong, JET_ColInfoList);
            if res != 0 {
                return Err(SimpleError::new(
                    format!("JetGetTableColumnInfoA failed with error {}", self.error_to_string(res))));
            }

            let subtable_id = col_list.assume_init().tableid;

            loop {
                let col_name = self.get_column_str(subtable_id, col_list.assume_init().columnidcolumnname, 255)?.unwrap();
                let col_id = self.get_column::<u32>(subtable_id, col_list.assume_init().columnidcolumnid)?.unwrap();
                let col_type = self.get_column::<u32>(subtable_id, col_list.assume_init().columnidcoltyp)?.unwrap();
                let col_cbmax = self.get_column::<u32>(subtable_id, col_list.assume_init().columnidcbMax)?.unwrap();
                let col_cp = self.get_column::<u16>(subtable_id, col_list.assume_init().columnidCp)?.unwrap();

                cols.push(ColumnInfo {
                    name: col_name,
                    id: col_id,
                    typ: col_type,
                    cbmax: col_cbmax,
                    cp: col_cp
                });

                if !self.move_row(subtable_id, JET_MoveNext) {
                    break;
                }
            }
        }
        self.close_table(table_id);
        Ok(cols)
    }
}

impl Drop for EseAPI {
    fn drop(&mut self) {
        if self.dbid > 0 {
            unsafe {
                JetCloseDatabase(self.sesid, self.dbid, 0);
            }
            self.dbid = 0;
        }
        if self.sesid > 0 {
            unsafe {
                JetDetachDatabaseA(self.sesid, std::ptr::null());
                JetEndSession(self.sesid, 0);
            }
            self.sesid = 0;
        }
        if self.instance > 0 {
            unsafe {
                JetTerm(self.instance);
            }
            self.instance = 0;
        }
    }
}

fn get_database_file_info(dbpath: &str) -> Result<JET_DBINFOMISC4, SimpleError> {
    let filename = CString::new(dbpath).unwrap();
    let db_info = MaybeUninit::<JET_DBINFOMISC4>::zeroed();
    let res_size = size_of::<JET_DBINFOMISC4>() as c_ulong;

    unsafe {
        let ptr: *mut c_void = db_info.as_ptr() as *mut c_void;
        let res = JetGetDatabaseFileInfoA(filename.as_ptr(), ptr, res_size, JET_DbInfoMisc);

        if JET_errSuccess == (res as u32) {
            Ok(*db_info.as_ptr())
        }
        else {
            Err(SimpleError::new(format!("JetGetDatabaseFileInfoA failed with error {}", res)))
        }
    }
}

fn set_system_parameter_l(paramId : u32, lParam: u64) -> bool {
    unsafe {
        let res = JetSetSystemParameterA(std::ptr::null_mut(), 0, paramId, lParam, std::ptr::null_mut()) as u32;
        res == JET_errSuccess
    }
}

fn set_system_parameter_s(paramId : u32, szParam: &str) -> bool {
    unsafe {
        let res = JetSetSystemParameterA(std::ptr::null_mut(), 0, paramId, 0, CString::new(szParam).unwrap().as_ptr()) as u32;
        res == JET_errSuccess
    }
}
