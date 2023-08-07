pub mod gen_db {
    #![cfg(all(feature = "nt_comparison", target_os = "windows"))]
    #![cfg(test)]

    use crate::ese_trait::ESE_CP;
    use crate::esent::esent::*;
    use encoding::{
        all::{ASCII, UTF_16LE, UTF_8},
        EncoderTrap, Encoding,
    };
    use std::convert::TryFrom;
    use std::{ffi::CString, fs, mem::size_of, os::raw, path::Path, ptr, str};

    macro_rules! jetcall {
    ($call:expr) => {
        unsafe {
            match $call {
                0 => Ok(()),
                err => Err(err),
            }
        }
    };
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
            EseAPI::set_system_parameter_l(JET_paramDisableCallbacks, true.into());
            EseAPI::set_system_parameter_sz(JET_paramRecovery, "Off");

            let mut instance: JET_INSTANCE = 0;
            jettry!(JetCreateInstanceA(
            &mut instance,
            instance_name.as_ptr() as *const i8
        ));
            jettry!(JetInit(&mut instance));

            let mut sesid: JET_SESID = 0;
            jettry!(JetBeginSessionA(
            instance,
            &mut sesid,
            ptr::null(),
            ptr::null()
        ));

            EseAPI {
                instance,
                sesid,
                dbid: 0,
            }
        }

        fn set_system_parameter_l(paramId: u32, lParam: u64) {
            jettry!(JetSetSystemParameterA(
            ptr::null_mut(),
            0,
            paramId,
            lParam,
            ptr::null_mut()
        ));
        }

        fn set_system_parameter_sz(paramId: u32, szParam: &str) {
            let strParam = CString::new(szParam).unwrap();
            jettry!(JetSetSystemParameterA(
            ptr::null_mut(),
            0,
            paramId,
            0,
            strParam.as_ptr()
        ));
        }

        fn create_column(
            name: &str,
            col_type: JET_COLTYP,
            cp: ESE_CP,
            grbit: JET_GRBIT,
        ) -> JET_COLUMNCREATE_A {
            println!("create_column: {}", name);

            JET_COLUMNCREATE_A {
                cbStruct: size_of::<JET_COLUMNCREATE_A>() as u32,
                szColumnName: CString::new(name).unwrap().into_raw(),
                coltyp: col_type,
                cbMax: 0,
                grbit,
                cp: cp as u32,
                pvDefault: ptr::null_mut(),
                cbDefault: 0,
                columnid: 0,
                err: 0,
            }
        }

        fn create_text_column(name: &str, cp: ESE_CP, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
            EseAPI::create_column(name, JET_coltypLongText, cp, grbit)
        }

        fn create_binary_column(name: &str, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
            EseAPI::create_column(name, JET_coltypLongBinary, ESE_CP::None, grbit)
        }

        fn create_table(
            self: &mut EseAPI,
            name: &str,
            columns: &mut Vec<JET_COLUMNCREATE_A>,
        ) -> JET_TABLEID {
            let mut table_def = JET_TABLECREATE_A {
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
                cCreated: 0,
            };

            println!("create_table: {}", name);
            jettry!(JetCreateTableColumnIndexA(
            self.sesid,
            self.dbid,
            &mut table_def
        ));
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

    pub fn prepare_db_gen(
        filename: &str,
        table: &str,
        pg_size: usize,
        record_size: usize,
        records_cnt: usize,
    ) -> std::path::PathBuf {
        let mut dst_path = std::env::temp_dir();
        dst_path.push(filename);

        if dst_path.exists() {
            let _ = fs::remove_file(&dst_path);
        }

        println!("creating {}", dst_path.display());
        let mut db_client = EseAPI::new(filename, pg_size);

        let dbpath = CString::new(dst_path.to_str().unwrap()).unwrap();
        jettry!(JetCreateDatabaseA(
        db_client.sesid,
        dbpath.as_ptr(),
        ptr::null(),
        &mut db_client.dbid,
        0
    ));

        let mut columns = Vec::<JET_COLUMNCREATE_A>::with_capacity(5);
        columns.push(EseAPI::create_text_column(
            "compressed_unicode",
            ESE_CP::Unicode,
            JET_bitColumnCompressed,
        ));
        columns.push(EseAPI::create_text_column(
            "compressed_ascii",
            ESE_CP::ASCII,
            JET_bitColumnCompressed,
        ));
        columns.push(EseAPI::create_binary_column(
            "compressed_binary",
            JET_bitColumnCompressed,
        ));
        columns.push(EseAPI::create_text_column(
            "usual_text",
            ESE_CP::None,
            JET_bitColumnTagged,
        ));

        let tableid = db_client.create_table(table, &mut columns);

        for i in 0..records_cnt {
            let s = format!("Record {number:>width$}", number = i, width = record_size);

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
                    ibLongValue: 0,
                    itagSequence: 0,
                    err: 0,
                };

                jettry!(JetSetColumns(db_client.sesid, tableid, &mut setColumn, 1));
            }

            jettry!(JetUpdate(
            db_client.sesid,
            tableid,
            ptr::null_mut(),
            0,
            ptr::null_mut()
        ));
            db_client.commit_transaction();
        }

        dst_path
    }

    pub fn clean_db_gen(dst_path: &Path) {
        fs::remove_file(dst_path.with_extension("jfm")).unwrap();
        fs::remove_file(dst_path).unwrap();
    }
}

#[cfg(test)]
mod test {
use crate::ese_parser::EseParser;
use crate::ese_parser::reader::ReadSeek;
use crate::ese_trait::*;
use crate::ese_parser::Reader;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use simple_error::SimpleError;

#[cfg(all(feature = "nt_comparison", target_os = "windows"))]
use crate::win::gen_db::*;

pub fn prepare_db(
    filename: &str,
    _table: &str,
    _pg_size: usize,
    _record_size: usize,
    _records_cnt: usize,
) -> PathBuf {
    let mut dst_path = PathBuf::from("testdata").canonicalize().unwrap();
    dst_path.push(filename);
    dst_path
}

#[test]
pub fn caching_test() -> Result<(), SimpleError> {
    let cache_size: usize = 10;
    let table = "test_table";
    let test_db = "decompress_test.edb";
    println!("db {}", test_db);
    let path = prepare_db(test_db, table, 1024 * 8, 1024, 1000);

    let file = File::open(path.clone()).unwrap();
    let buf_reader = BufReader::with_capacity(4096, file);

    let reader = Reader::new(buf_reader, cache_size as usize)?;
    let page_size = reader.page_size() as u64;
    let num_of_pages =
        std::cmp::min(fs::metadata(&path).unwrap().len() / page_size, page_size) as usize;
    let full_cache_size = 6 * cache_size;
    let stride = num_of_pages / full_cache_size;
    let chunk_size = page_size as usize / num_of_pages;
    let mut chunks = Vec::<Vec<u8>>::with_capacity(stride as usize);

    println!(
        "cache_size: {}, page_size: {}, num_of_pages: {}, stride: {}, chunk_size: {}",
        cache_size, page_size, num_of_pages, stride, chunk_size
    );

    for pass in 1..3 {
        for pg_no in 1_u32..12_u32 {
            let offset: u64 = pg_no as u64 * (page_size + chunk_size as u64);

            println!("pass {}, pg_no {}, offset {:x} ", pass, pg_no, offset);
            let mut chunk = Vec::<u8>::with_capacity(stride as usize);

            if pass == 1 {
                assert!(!reader.cache().get_mut().contains_key(&pg_no));
                reader.read(offset, &mut chunk)?;
                chunks.push(chunk);
            } else {
                // pg_no == 1 was deleted, because cache_size is 10 pages
                // and we read 11, so least recently used page (1) was deleted
                assert_eq!(reader.cache().get_mut().contains_key(&pg_no), pg_no != 1);
                reader.read(offset, &mut chunk)?;
                assert_eq!(chunk, chunks[pg_no as usize - 1]);
            }
        }
    }
    Ok(())
}

#[cfg(all(feature = "nt_comparison", target_os = "windows"))]
#[test]
pub fn caching_test_windows() -> Result<(), SimpleError> {
    let cache_size: usize = 10;
    let table = "test_table";
    let test_db = "caching_test.edb";
    println!("db {}", test_db);

    let path = prepare_db_gen(test_db, table, 1024 * 8, 1024, 1000);
    let file = File::open(path.clone()).unwrap();
    let buf_reader = BufReader::with_capacity(4096, file);

    let reader = Reader::new(buf_reader, cache_size as usize)?;
    let page_size = reader.page_size() as u64;
    let num_of_pages =
        std::cmp::min(fs::metadata(&path).unwrap().len() / page_size, page_size) as usize;
    let full_cache_size = 6 * cache_size;
    let stride = num_of_pages / full_cache_size;
    let chunk_size = page_size as usize / num_of_pages;
    let mut chunks = Vec::<Vec<u8>>::with_capacity(stride as usize);

    println!(
        "cache_size: {}, page_size: {}, num_of_pages: {}, stride: {}, chunk_size: {}",
        cache_size, page_size, num_of_pages, stride, chunk_size
    );

    for pass in 1..3 {
        for pg_no in 1_u32..12_u32 {
            let offset: u64 = pg_no as u64 * (page_size + chunk_size as u64);

            println!("pass {}, pg_no {}, offset {:x} ", pass, pg_no, offset);
            let mut chunk = Vec::<u8>::with_capacity(stride as usize);

            if pass == 1 {
                assert!(!reader.cache().get_mut().contains_key(&pg_no));
                reader.read(offset, &mut chunk)?;
                chunks.push(chunk);
            } else {
                // pg_no == 1 was deleted, because cache_size is 10 pages
                // and we read 11, so least recently used page (1) was deleted
                assert_eq!(reader.cache().get_mut().contains_key(&pg_no), pg_no != 1);
                reader.read(offset, &mut chunk)?;
                assert_eq!(chunk, chunks[pg_no as usize - 1]);
            }
        }
    }
    clean_db_gen(&path);
    Ok(())
}

fn check_row<R: ReadSeek>(
    jdb: &mut EseParser<R>,
    table_id: u64,
    columns: &[ColumnInfo],
) -> HashSet<String> {
    let mut values = HashSet::<String>::new();
    for col in columns {
        match jdb.get_column_str(table_id, col.id, col.cp) {
            Ok(result) => {
                if let Some(value) = result {
                    values.insert(value);
                } else {
                    values.insert("".to_string());
                }
            }
            Err(e) => panic!("error: {}", e),
        }
    }
    values
}

#[test]
pub fn decompress_test_7bit() -> Result<(), SimpleError> {
    // if record size < 1024 - 7 bit compression is used
    run_decompress_test("decompress_test.edb", 10)?;
    Ok(())
}

#[test]
pub fn decompress_test_lzxpress() -> Result<(), SimpleError> {
    // if record size > 1024 - lzxpress compression is used
    run_decompress_test("decompress_test2.edb", 2048)?;
    Ok(())
}

pub fn run_decompress_test(filename: &str, record_size: usize) -> Result<(), SimpleError> {
    let table = "test_table";
    let path = prepare_db(filename, table, 1024 * 8, record_size, 10);
    //let mut jdb = EseParser::init(5);
    let mut jdb = EseParser::load_from_path(5, path.to_str().unwrap())?;

    /*match jdb.load(path.to_str().unwrap()) {
        Some(e) => panic!("Error: {}", e),
        None => println!("Loaded {}", path.display())
    }*/

    let table_id = jdb.open_table(table)?;
    let columns = jdb.get_columns(table)?;

    assert!(jdb.move_row(table_id, ESE_MoveFirst)?);

    for i in 0.. {
        let values = check_row(&mut jdb, table_id, &columns);
        assert_eq!(values.len(), 1);
        let v = format!("Record {number:>width$}", number = i, width = record_size);
        assert!(values.contains(&v), "{}", true);
        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    Ok(())
}
}