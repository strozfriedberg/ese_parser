//#![cfg(test)]

use md5;
use std::path::PathBuf;
use std::fs;

// #[cfg(target_os = "windows")]
// use crate::parser::reader::gen_db::*;

/*
#[cfg(target_os = "windows")]
#[test]

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
        let strParam = CString::new(szParam).unwrap();
        jettry!(JetSetSystemParameterA(ptr::null_mut(), 0, paramId, 0, strParam.as_ptr()));
    }

    fn create_column(name: &str, col_type: JET_COLTYP, cp: ESE_CP, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        println!("create_column: {}", name);

        JET_COLUMNCREATE_A{
            cbStruct: size_of::<JET_COLUMNCREATE_A>() as u32,
            szColumnName: CString::new(name).unwrap().into_raw(),
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
                    szTableName: CString::new(name).unwrap().into_raw(),
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
pub fn get_esent



pub fn get_parser
*/


pub fn md5_digest(input: String) -> u64 {
    let digest = md5::compute(input);
    format!("{:x}",digest)
}

pub fn get_file_contents(filename: &str) -> String {
    let mut dst_path = PathBuf::from("testdata").canonicalize().unwrap();
    dst_path.push(filename);
    let contents = fs::read_to_string(dst_path).unwrap();
    contents
}

#[test]
fn test_compare_output() {
    let esent_file = "esentoutput.txt";
    let parser_file = "parseroutput.txt";
    let esent_hash_input = get_file_contents(esent_file);
    let parser_hash_input = get_file_contents(parser_file);

    let esent_digest = md5_digest(esent_hash_input);
    let parser_digest = md5_digest(parser_hash_input);

    assert_eq!(esent_digest, parser_digest);
}