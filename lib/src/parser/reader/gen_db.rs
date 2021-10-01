#![cfg(target_os = "windows")]
#![cfg(test)]

use super::*;
use std::{str, ffi::CString, mem::size_of, os::raw, ptr, path::PathBuf};
use crate::esent::esent::*;
use encoding::{all::{ASCII, UTF_16LE, UTF_8}, Encoding, EncoderTrap};
use crate::ese_trait::ESE_CP;
use std::convert::TryFrom;

macro_rules! jetcall {
    ($call:expr) => {
        unsafe {
            match $call {
                0 => Ok(()),
                err => Err(err),
            }
        }
    }
}

macro_rules! jettry {
    ($func:ident($($args:expr),*)) => {
        match jetcall!($func($($args),*)) {
            Ok(x) => x,
            Err(e) => panic!("{} failed: {}", stringify!($func), e),
        }
    }
}

#[derive(Debug)]
pub struct EseAPI {
    instance: JET_INSTANCE,
    sesid: JET_SESID,
    dbid: JET_DBID,
}

impl EseAPI {
    fn new(instance_name: &str, pg_size: usize) -> EseAPI {
        EseAPI::set_system_parameter_l(JET_paramDatabasePageSize, pg_size as u64);
        EseAPI::set_system_parameter_l(JET_paramDisableCallbacks, (true as u64).into());
        EseAPI::set_system_parameter_sz(JET_paramRecovery, "Off");

        let mut instance : JET_INSTANCE = 0;
        jettry!(JetCreateInstanceA(&mut instance, instance_name.as_ptr() as *const i8));
        jettry!(JetInit(&mut instance));

        let mut sesid : JET_SESID = 0;
        jettry!(JetBeginSessionA(instance, &mut sesid, ptr::null(), ptr::null()));

        EseAPI { instance, sesid, dbid: 0 }
    }

    fn set_system_parameter_l(paramId : u32, lParam: u64) {
        jettry!(JetSetSystemParameterA(ptr::null_mut(), 0, paramId, lParam, ptr::null_mut()));
    }

    fn set_system_parameter_sz(paramId : u32, szParam: &str) {
        let strParam = CString::new(szParam).expect("String param failed");
        jettry!(JetSetSystemParameterA(ptr::null_mut(), 0, paramId, 0, strParam.as_ptr()));
    }

    fn create_column(name: &str, col_type: JET_COLTYP, cp: ESE_CP, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        println!("create_column: {}", name);

        JET_COLUMNCREATE_A{
            cbStruct: size_of::<JET_COLUMNCREATE_A>() as u32,
            szColumnName: CString::new(name)
                .map_err(|e: std::ffi::NulError | SimpleError::new(e.to_string()))?
                .into_raw(),
            coltyp: col_type,
            cbMax: 0,
            grbit,
            cp: cp as u32,
            pvDefault: ptr::null_mut(), cbDefault: 0, columnid: 0, err: 0 }
    }

    fn create_text_column(name: &str, cp: ESE_CP, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        EseAPI::create_column(name, JET_coltypLongText, cp, grbit)
    }

    fn create_binary_column(name: &str, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        EseAPI::create_column(name, JET_coltypLongBinary, ESE_CP::None, grbit)
    }

    fn create_table(self: &mut EseAPI,
                    name: &str,
                    columns: &mut Vec<JET_COLUMNCREATE_A>) -> JET_TABLEID {

        let mut table_def =  JET_TABLECREATE_A{
                    cbStruct: size_of::<JET_TABLECREATE_A>() as u32,
                    szTableName: CString::new(name)
                        .map_err(|e: std::ffi::NulError | SimpleError::new(e.to_string()))?
                        .into_raw(),
                    szTemplateTableName: ptr::null_mut(),
                    ulPages: 0,
                    ulDensity: 0,
                    rgcolumncreate: columns.as_mut_ptr(),
                    cColumns: columns.len() as raw::c_ulong,
                    rgindexcreate: ptr::null_mut(),
                    cIndexes: 0,
                    grbit: 0,
                    tableid: 0,
                    cCreated: 0
                };

        println!("create_table: {}", name);
        jettry!(JetCreateTableColumnIndexA(self.sesid, self.dbid, &mut table_def ));
        table_def.tableid
    }

    fn begin_transaction(self: &EseAPI) {
        jettry!(JetBeginTransaction(self.sesid));
    }

    fn commit_transaction(self: &EseAPI) {
        jettry!(JetCommitTransaction(self.sesid, 0));
    }
}

impl Drop for EseAPI {
    fn drop(&mut self) {
        println!("Dropping EseAPI");

        if self.sesid != 0 {
            jettry!(JetEndSession(self.sesid, 0));
        }
        if self.instance != 0 {
            jettry!(JetTerm2(self.instance, JET_bitTermComplete));
        }
    }
}

pub fn prepare_db(filename: &str, table: &str, pg_size: usize, record_size: usize, records_cnt: usize)
    -> PathBuf {
    let mut dst_path = std::env::temp_dir();
    dst_path.push(filename);
    //I think the error handling should be right here, and then 146 can be shortened
    if dst_path.exists() {
        let _ = fs::remove_file(&dst_path);
    }

    println!("creating {}", dst_path.display());
    let mut db_client = EseAPI::new(filename, pg_size);

    let dbpath = CString::new(dst_path.to_str().unwrap()).map_err(|e: std::ffi::NulError | SimpleError::new(e.to_string()))?; //hmm 
    jettry!(JetCreateDatabaseA(db_client.sesid, dbpath.as_ptr(), ptr::null(), &mut db_client.dbid, 0));

    let mut columns = Vec::<JET_COLUMNCREATE_A>::with_capacity(5);
    columns.push(EseAPI::create_text_column("compressed_unicode", ESE_CP::Unicode, JET_bitColumnCompressed));
    columns.push(EseAPI::create_text_column("compressed_ascii", ESE_CP::ASCII, JET_bitColumnCompressed));
    columns.push(EseAPI::create_binary_column("compressed_binary", JET_bitColumnCompressed));
    columns.push(EseAPI::create_text_column("usual_text", ESE_CP::None, JET_bitColumnTagged));

    let tableid = db_client.create_table(table, &mut columns);

    for i in 0..records_cnt {
        let s = format!("Record {number:>width$}", number=i, width=record_size);

        db_client.begin_transaction();

        jettry!(JetPrepareUpdate(db_client.sesid, tableid, JET_prepInsert));
        for col in &columns {
            let data = match ESE_CP::try_from(col.cp as u16) {
                Ok(ESE_CP::Unicode) => match UTF_16LE.encode(&s, EncoderTrap::Strict) {
                    Ok(data) => data,
                    Err(e) => panic!("{}", e),
                },
                Ok(ESE_CP::ASCII) => match ASCII.encode(&s, EncoderTrap::Strict) {
                    Ok(data) => data,
                    Err(e) => panic!("{}", e),
                },
                Ok(ESE_CP::None) => match UTF_8.encode(&s, EncoderTrap::Strict) {
                    Ok(data) => data,
                    Err(e) => panic!("{}", e),
                },
                Err(e) => panic!("{:?}", e),
            };

            let mut setColumn = JET_SETCOLUMN {
                columnid: col.columnid,
                pvData: data.as_ptr() as *const raw::c_void,
                cbData: data.len() as raw::c_ulong,
                grbit: col.grbit,
                ibLongValue: 0, itagSequence: 0, err: 0 };

            jettry!(JetSetColumns(db_client.sesid, tableid, &mut setColumn, 1));
        }

        jettry!(JetUpdate(db_client.sesid, tableid, ptr::null_mut(), 0, ptr::null_mut()));
        db_client.commit_transaction();
    }

    dst_path
}

pub fn clean_db(dst_path: &PathBuf) {
    fs::remove_file(dst_path.with_extension("jfm")).map_err(remove_file)?;
    fs::remove_file(dst_path).map_err(remove_file)?;
}
