use super::*;
use std::{str, ffi::CString, ptr::null_mut, convert::TryFrom, collections::HashSet};
use crate::esent::*;
use crate::ese_parser;
use crate::ese_trait::EseDb;
use encoding::{all::{ASCII, UTF_16LE, UTF_8}, Encoding, EncoderTrap, DecoderTrap};

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

fn size_of<T> () -> raw::c_ulong{
    mem::size_of::<T>() as raw::c_ulong
}

#[derive(Debug)]
pub struct EseAPI {
    instance: JET_INSTANCE,
    sesid: JET_SESID,
    dbid: JET_DBID,
}

enum JET_CP {
    None = 0,
    Unicode = 1200,
    ASCII = 1252
}

impl TryFrom<u32> for JET_CP {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == JET_CP::None as u32 => Ok(JET_CP::None),
            x if x == JET_CP::ASCII as u32 => Ok(JET_CP::ASCII),
            x if x == JET_CP::Unicode as u32 => Ok(JET_CP::Unicode),
            _ => Err(()),
        }
    }
}

impl EseAPI {
    fn new(pg_size: usize) -> EseAPI {
        EseAPI::set_system_parameter_l(JET_paramDatabasePageSize, pg_size as u64);
        EseAPI::set_system_parameter_l(JET_paramDisableCallbacks, (true as u64).into());
        EseAPI::set_system_parameter_sz(JET_paramRecovery, "Off");

        let mut instance : JET_INSTANCE = 0;
        jettry!(JetCreateInstanceA(&mut instance, ptr::null()));
        jettry!(JetInit(&mut instance));

        let mut sesid : JET_SESID = 0;
        jettry!(JetBeginSessionA(instance, &mut sesid, ptr::null(), ptr::null()));

        EseAPI { instance, sesid, dbid: 0 }
    }

    fn set_system_parameter_l(paramId : u32, lParam: u64) {
        jettry!(JetSetSystemParameterA(ptr::null_mut(), 0, paramId, lParam, ptr::null_mut()));
    }

    fn set_system_parameter_sz(paramId : u32, szParam: &str) {
        jettry!(JetSetSystemParameterA(ptr::null_mut(), 0, paramId, 0, CString::new(szParam).unwrap().as_ptr()));
    }

    fn create_column(name: &str, col_type: JET_COLTYP, cp: JET_CP, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        println!("create_column: {}", name);

        JET_COLUMNCREATE_A{
            cbStruct: size_of::<JET_COLUMNCREATE_A>(),
            szColumnName: CString::new(name).unwrap().into_raw(),
            coltyp: col_type,
            cbMax: 0,
            grbit,
            cp: cp as u32,
            pvDefault: ptr::null_mut(), cbDefault: 0, columnid: 0, err: 0 }
    }

    fn create_num_column(name: &str, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        EseAPI::create_column(name, JET_coltypLong, JET_CP::None, grbit)
    }

    fn create_text_column(name: &str, cp: JET_CP, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        EseAPI::create_column(name, JET_coltypLongText, cp, grbit)
    }

    fn create_binary_column(name: &str, grbit: JET_GRBIT) -> JET_COLUMNCREATE_A {
        EseAPI::create_column(name, JET_coltypLongBinary, JET_CP::None, grbit)
    }

    fn create_table(self: &mut EseAPI,
                    name: &str,
                    columns: &mut Vec<JET_COLUMNCREATE_A>) -> JET_TABLEID {

        let mut table_def =  JET_TABLECREATE_A{
                    cbStruct: size_of::<JET_TABLECREATE_A>(),
                    szTableName: CString::new(name).unwrap().into_raw(),
                    szTemplateTableName: ptr::null_mut(),
                    ulPages: 0,
                    ulDensity: 0,
                    rgcolumncreate: columns.as_mut_ptr(),
                    cColumns: columns.len() as raw::c_ulong,
                    rgindexcreate: null_mut(),
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

fn prepare_db(filename: &str, table: &str, pg_size: usize, record_size: usize, records_cnt: usize) -> PathBuf {
    let mut dst_path = PathBuf::from("testdata").canonicalize().unwrap();
    dst_path.push(filename);

    if dst_path.exists() {
        let _ = fs::remove_file(&dst_path);
    }

    println!("creating {}", dst_path.display());
    let mut db_client = EseAPI::new(pg_size);

    let dbpath = CString::new(dst_path.to_str().unwrap()).unwrap();
    jettry!(JetCreateDatabaseA(db_client.sesid, dbpath.as_ptr(), ptr::null(), &mut db_client.dbid, 0));

    let mut columns = Vec::<JET_COLUMNCREATE_A>::with_capacity(5);
    //columns.push(EseAPI::create_num_column("PK",JET_bitColumnAutoincrement));
    columns.push(EseAPI::create_text_column("compressed_unicode", JET_CP::Unicode, JET_bitColumnCompressed));
    columns.push(EseAPI::create_text_column("compressed_ascii", JET_CP::ASCII, JET_bitColumnCompressed));
    columns.push(EseAPI::create_binary_column("compressed_binary", JET_bitColumnCompressed));
    columns.push(EseAPI::create_text_column("usual_text", JET_CP::None, JET_bitColumnTagged));

    let tableid = db_client.create_table(table, &mut columns);

    for i in 0..records_cnt {
        let s = format!("Record {number:>width$}", number=i, width=record_size);

        db_client.begin_transaction();

        jettry!(JetPrepareUpdate(db_client.sesid, tableid, JET_prepInsert));
        for col in &columns {
            let data = match col.cp.try_into() {
                Ok(JET_CP::Unicode) => match UTF_16LE.encode(&s, EncoderTrap::Strict) {
                    Ok(data) => data,
                    Err(e) => panic!("{}", e),
                },
                Ok(JET_CP::ASCII) => match ASCII.encode(&s, EncoderTrap::Strict) {
                    Ok(data) => data,
                    Err(e) => panic!("{}", e),
                },
                Ok(JET_CP::None) => match UTF_8.encode(&s, EncoderTrap::Strict) {
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

            //println!("'{}' {}", s, s.len());

            jettry!(JetSetColumns(db_client.sesid, tableid, &mut setColumn, 1));
        }

        jettry!(JetUpdate(db_client.sesid, tableid, ptr::null_mut(), 0, ptr::null_mut()));
        db_client.commit_transaction();
    }

    dst_path
}

#[test]
pub fn caching_test() -> Result<(), SimpleError> {
    let cache_size: usize = 10;
    let table = "test_table";
    let path = prepare_db("caching_test.edb", table, 1024 * 8, 1024, 4000);
    let mut reader = Reader::new(&path, cache_size as usize)?;
    let page_size = reader.page_size;
    let num_of_pages = std::cmp::min(fs::metadata(&path).unwrap().len() / page_size, page_size) as usize;
    let full_cache_size = 6 * cache_size;
    let stride = num_of_pages / full_cache_size;
    let chunk_size = page_size as usize / num_of_pages;
    let mut chunks = Vec::<Vec<u8>>::with_capacity(stride as usize);

    println!("cache_size: {}, page_size: {}, num_of_pages: {}, stride: {}, chunk_size: {}",
        cache_size, page_size, num_of_pages, stride, chunk_size);

    for pass in 1..3 {
        for pg_no in 1_usize..12_usize {
            let offset: u64 = (pg_no * (page_size as usize + chunk_size)) as u64;

            println!("pass {}, pg_no {}, offset {:x} ", pass, pg_no, offset);

            if pass == 1 {
                let mut chunk = Vec::<u8>::with_capacity(stride as usize);
                assert!(!reader.cache.get_mut().contains_key(&pg_no as &usize));
                reader.read(offset, &mut chunk)?;
                chunks.push(chunk);
            } else {
                let mut chunk = Vec::<u8>::with_capacity(stride as usize);
                // pg_no == 1 was deleted, because cache_size is 10 pages
                // and we read 11, so least recently used page (1) was deleted
                assert_eq!(reader.cache.get_mut().contains_key(&pg_no), pg_no != 1);
                reader.read(offset, &mut chunk)?;
                assert_eq!(chunk, chunks[pg_no as usize - 1]);
            }
        }
    }
    Ok(())
}

#[test]
pub fn decompress_test() -> Result<(), SimpleError> {
   let table = "test_table";
   let path = prepare_db("decompress_test.edb", table, 1024 * 8, 10, 10);
   let mut jdb : ese_parser::EseParser = ese_parser::EseParser::init(5);

   match jdb.load(&path.to_str().unwrap()) {
        Some(e) => panic!("Error: {}", e),
        None => println!("Loaded {}", path.display())
   }

   let table_id = jdb.open_table(&table)?;
   let columns = jdb.get_columns(&table)?;
   let mut values = HashSet::<String>::new();

   for col in columns {
       print!("{}: ", col.name);
       match jdb.get_column_str(table_id, col.id, 0) {
           Ok(result) =>
               if let Some(mut value) = result {
                   if col.cp == JET_CP::Unicode as u16 {
                       unsafe {
                           let buffer = slice::from_raw_parts(value.as_bytes() as *const _ as *const u16, value.len() / 2);
                           value = String::from_utf16(&buffer).unwrap();
                       }
                   }
                   if let Ok(s) = UTF_8.decode(&value.as_bytes(), DecoderTrap::Strict) {
                       value = s;
                   }
                   println!("{}", value);
                   values.insert(value);
               }
               else {
                   println!("column '{}' has no value", col.name);
                   values.insert("".to_string());
               },
           Err(e) => panic!("error: {}", e),
       }
   }

   println!("values: {:?}", values);
   assert_eq!(values.len(), 1);

   Ok(())
}

