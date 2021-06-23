#[cfg(target_os = "windows")]

use crate::ese_trait::*;
use crate::esent::esent::*;

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
            let err = JetGetColumnInfoA(self.sesid, self.dbid, tbl.as_ptr(),
                col.as_ptr(), col_base.as_mut_ptr() as *mut c_void, size_of::<JET_COLUMNBASE_A>() as c_ulong, JET_ColInfoBase);
            if err != 0 {
                return Err(SimpleError::new(
                    format!("JetOpenDatabaseA failed with error {}", self.error_to_string(err))));
            }
            Ok(col_base.assume_init())
        }
    }

    fn get_database_file_info(dbpath: &str) -> Result<JET_DBINFOMISC4, SimpleError> {
        let filename = CString::new(dbpath).unwrap();
        let db_info = MaybeUninit::<JET_DBINFOMISC4>::zeroed();
        let res_size = size_of::<JET_DBINFOMISC4>() as c_ulong;

        unsafe {
            let ptr: *mut c_void = db_info.as_ptr() as *mut c_void;
            let err = JetGetDatabaseFileInfoA(filename.as_ptr(), ptr, res_size, JET_DbInfoMisc);

            if JET_errSuccess == (err as u32) {
                Ok(*db_info.as_ptr())
            }
            else {
                Err(SimpleError::new(format!("JetGetDatabaseFileInfoA failed with error {}", err)))
            }
        }
    }

    fn set_system_parameter_l(paramId : u32, lParam: u64) -> bool {
        unsafe {
            let err = JetSetSystemParameterA(std::ptr::null_mut(), 0, paramId, lParam, std::ptr::null_mut()) as u32;
            err == JET_errSuccess
        }
    }

    fn set_system_parameter_sz(paramId : u32, szParam: &str) -> bool {
        let strParam = CString::new(szParam).unwrap();
        unsafe {
            let err = JetSetSystemParameterA(std::ptr::null_mut(), 0, paramId, 0, strParam.as_ptr()) as u32;
            err == JET_errSuccess
        }
    }

    fn get_column_dyn_helper(&self, table: u64, column: u32, data: &mut[u8], size: usize)
        -> Result<u32, SimpleError> {
        let mut bytes : c_ulong = 0;
        unsafe {
            let err = JetRetrieveColumn(self.sesid, table, column, data.as_mut_ptr() as *mut c_void, size as u32,
                &mut bytes, 0, std::ptr::null_mut());
            if err != 0 {
                if err == JET_wrnColumnNull as i32 {
                    return Ok(0);
                }
                return Err(SimpleError::new(
                    format!("JetRetrieveColumn failed with error {}", self.error_to_string(err))));
            }

            Ok(bytes as u32)
        }
    }

    pub fn get_fixed_column<T>(&self, table: u64, column: u32) -> Result<Option<T>, SimpleError> {
        let size : c_ulong = size_of::<T>() as u32;
        let mut v = MaybeUninit::<T>::zeroed();

        unsafe {
            self.get_column_dyn_helper(table, column,
                std::slice::from_raw_parts_mut::<u8>(v.as_mut_ptr() as *mut u8, size as usize), size as usize)?;
            Ok(Some(v.assume_init()))
        }
    }

    pub fn init() -> EseAPI {
        EseAPI { instance: 0, sesid: 0, dbid: 0 }
    }
}

impl EseDb for EseAPI {

    fn load(&mut self, dbpath: &str) -> Option<SimpleError> {
        let dbinfo = match EseAPI::get_database_file_info(dbpath) {
            Ok(i) => i,
            Err(e) => return Some(e)
        };
        EseAPI::set_system_parameter_l(JET_paramDatabasePageSize, (dbinfo.cbPageSize as u64).into());
        EseAPI::set_system_parameter_l(JET_paramDisableCallbacks, (true as u64).into());
        EseAPI::set_system_parameter_sz(JET_paramRecovery, "Off");

        let mut instance : JET_INSTANCE = 0;
        unsafe {
            let err = JetCreateInstanceA(&mut instance, std::ptr::null());
            if err != 0 {
                return Some(SimpleError::new(format!("JetCreateInstanceA failed with error: {}", err)));
            }
        }
        unsafe {
            let err = JetInit(&mut instance);
            if err != 0 {
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetInit failed with error {}", err)));
            }
        }

        let mut sesid : JET_SESID = 0;
        unsafe {
            let err = JetBeginSessionA(instance, &mut sesid, std::ptr::null(), std::ptr::null() );
            if err != 0 {
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetBeginSessionA failed with error {}", err)));
            }
        }

        let dbpath = CString::new(dbpath).unwrap();
        unsafe {
            let err = JetAttachDatabaseA(sesid, dbpath.as_ptr(), JET_bitDbReadOnly);
            if err != 0 {
                JetEndSession(sesid, 0);
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetAttachDatabaseA failed with error {}", err)));
            }
        }

        let mut dbid : JET_DBID = 0;
        unsafe {
            let err = JetOpenDatabaseA(sesid, dbpath.as_ptr(), std::ptr::null(), &mut dbid, JET_bitDbReadOnly);
            if err != 0 {
                JetDetachDatabaseA(sesid, std::ptr::null());
                JetEndSession(sesid, 0);
                JetTerm(instance);
                return Some(SimpleError::new(format!("JetOpenDatabaseA failed with error {}", err)));
            }
        }

        self.instance = instance;
        self.sesid = sesid;
        self.dbid = dbid;
        None
    }

    fn error_to_string(&self, err: i32) -> String {
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

    fn open_table(&self, table: &str) -> Result<u64, SimpleError> {
        let tbl = CString::new(table).unwrap();
        let mut tableid : JET_TABLEID = 0;
        unsafe {
            let err = JetOpenTableA(self.sesid, self.dbid, tbl.as_ptr(), std::ptr::null(), 0, JET_bitTableReadOnly,
                &mut tableid);
            if err != 0 {
                return Err(SimpleError::new(format!("JetOpenTableA failed with error {}", self.error_to_string(err))));
            }
            Ok(tableid)
        }
    }

    fn close_table(&self, table: u64) -> bool {
        unsafe {
            let err = JetCloseTable(self.sesid, table);
            err == 0
        }
    }

    fn get_column(&self, table: u64, column: u32) -> Result< Option<Vec<u8>>, SimpleError> {
        let mut vres : Vec<u8> = Vec::new();

        loop {
            let mut v : Vec<u8> = Vec::new();
            let size: usize = 4096;
            v.resize(size, 0);

            let mut bytes : c_ulong = 0;
            unsafe {
                let mut retinfo = JET_RETINFO {
                    cbStruct: size_of::<JET_RETINFO>() as c_ulong,
                    ibLongValue: vres.len() as c_ulong,
                    itagSequence : 1,
                    columnidNextTagged: 0
                };
                let err = JetRetrieveColumn(self.sesid, table, column, v.as_mut_slice().as_mut_ptr() as *mut c_void, size as u32,
                    &mut bytes, 0, &mut retinfo);
                if err != 0 && err != JET_wrnBufferTruncated as i32 {
                    if err == JET_wrnColumnNull as i32 {
                        return Ok(None);
                    }
                    return Err(SimpleError::new(
                        format!("JetRetrieveColumn failed with error {}", self.error_to_string(err))));
                }
                if bytes == 0 {
                    return Ok(None);
                }

                v.truncate(bytes as usize);
                vres.append(v.as_mut());
                if err != JET_wrnBufferTruncated as i32 {
                    break;
                }
            }
        }
        Ok(Some(vres))
    }

    fn get_column_mv(&self, table: u64, column: u32, multi_value_index: u32)
        -> Result< Option<Vec<u8>>, SimpleError> {
            let mut vres : Vec<u8> = Vec::new();

            loop {
                let mut v : Vec<u8> = Vec::new();
                let size: usize = 4096;
                v.resize(size, 0);

                let mut bytes : c_ulong = 0;
                unsafe {
                    let mut retinfo = JET_RETINFO {
                        cbStruct: size_of::<JET_RETINFO>() as c_ulong,
                        ibLongValue: vres.len() as c_ulong,
                        itagSequence : multi_value_index,
                        columnidNextTagged: 0
                    };
                    let err = JetRetrieveColumn(self.sesid, table, column, v.as_mut_slice().as_mut_ptr() as *mut c_void, size as u32,
                        &mut bytes, 0, &mut retinfo);
                    if err != 0 && err != JET_wrnBufferTruncated as i32 {
                        if err == JET_wrnColumnNull as i32 {
                            return Ok(None);
                        }
                        return Err(SimpleError::new(
                            format!("JetRetrieveColumn failed with error {}", self.error_to_string(err))));
                    }

                    v.truncate(bytes as usize);
                    vres.append(v.as_mut());

                    if err == JET_wrnBufferTruncated as i32 {
                        continue;
                    } else {
                        break;
                    }
                }
            }
            Ok(Some(vres))
    }

    fn move_row(&self, table: u64, crow: i32) -> bool {
        unsafe {
            let err = JetMove(self.sesid, table, crow as std::os::raw::c_long, 0);
            err == 0
        }
    }

    fn get_tables(&self) -> Result<Vec<String>, SimpleError> {
        let c_name_info = self.get_column_info("MSysObjects", "Name")?;
        let c_type_info = self.get_column_info("MSysObjects", "Type")?;

        let table_id = self.open_table("MSysObjects")?;

        let mut err : Vec<String> = Vec::new();
        loop {
            let name_str = self.get_column_str(table_id, c_name_info.columnid, c_name_info.cp)?.unwrap();
            let type_word = self.get_fixed_column::<u16>(table_id, c_type_info.columnid)?.unwrap();

            if type_word == 1 {
               err.push(name_str);
            }

            if !self.move_row(table_id, ESE_MoveNext) {
                break;
            }
        }

        self.close_table(table_id);
        Ok(err)
    }

    fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>, SimpleError> {
        let table_id = self.open_table(table)?;
        let mut cols : Vec<ColumnInfo> = Vec::new();
        let mut col_list = MaybeUninit::<JET_COLUMNLIST>::zeroed();
        unsafe {
            let err = JetGetTableColumnInfoA(self.sesid, table_id, std::ptr::null(),
                col_list.as_mut_ptr() as *mut c_void, size_of::<JET_COLUMNLIST>() as c_ulong, JET_ColInfoList);
            if err != 0 {
                return Err(SimpleError::new(
                    format!("JetGetTableColumnInfoA failed with error {}", self.error_to_string(err))));
            }

            let subtable_id = col_list.assume_init().tableid;

            loop {
                let col_name = self.get_column_str(subtable_id, col_list.assume_init().columnidcolumnname, 0)?.unwrap();
                let col_id = self.get_fixed_column::<u32>(subtable_id, col_list.assume_init().columnidcolumnid)?.unwrap();
                let col_type = self.get_fixed_column::<u32>(subtable_id, col_list.assume_init().columnidcoltyp)?.unwrap();
                let col_cbmax = self.get_fixed_column::<u32>(subtable_id, col_list.assume_init().columnidcbMax)?.unwrap();
                let col_cp = self.get_fixed_column::<u16>(subtable_id, col_list.assume_init().columnidCp)?.unwrap();

                cols.push(ColumnInfo {
                    name: col_name,
                    id: col_id,
                    typ: col_type,
                    cbmax: col_cbmax,
                    cp: col_cp
                });

                if !self.move_row(subtable_id, ESE_MoveNext) {
                    break;
                }
            }
        }
        self.close_table(table_id);
        cols.sort_by(|a, b| a.id.cmp(&b.id));
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
