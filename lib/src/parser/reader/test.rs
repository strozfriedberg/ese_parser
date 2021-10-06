#![cfg(test)]

use super::*;
use std::collections::HashSet;
use crate::ese_parser::EseParser;
use crate::ese_trait::*;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use crate::parser::reader::gen_db::*;

pub fn prepare_db(filename: &str, _table: &str, _pg_size: usize, _record_size: usize, _records_cnt: usize) -> PathBuf {
    let mut dst_path = PathBuf::from("testdata").canonicalize().unwrap();
    dst_path.push(filename);
    dst_path
}

pub fn clean_db(_dst_path: &Path) {
}

#[test]
pub fn caching_test() -> Result<(), SimpleError> {
    let cache_size: usize = 10;
    let table = "test_table";
    let test_db = "decompress_test.edb";
    println!("db {}", test_db);
    let path = prepare_db(test_db, table, 1024 * 8, 1024, 1000);
    let mut reader = Reader::new(&path, cache_size as usize)?;
    let page_size = reader.page_size as u64;
    let num_of_pages = std::cmp::min(fs::metadata(&path).unwrap().len() / page_size, page_size) as usize;
    let full_cache_size = 6 * cache_size;
    let stride = num_of_pages / full_cache_size;
    let chunk_size = page_size as usize / num_of_pages;
    let mut chunks = Vec::<Vec<u8>>::with_capacity(stride as usize);

    println!("cache_size: {}, page_size: {}, num_of_pages: {}, stride: {}, chunk_size: {}",
        cache_size, page_size, num_of_pages, stride, chunk_size);

    for pass in 1..3 {
        for pg_no in 1_u32..12_u32 {
            let offset: u64 = pg_no as u64 * (page_size + chunk_size as u64);

            println!("pass {}, pg_no {}, offset {:x} ", pass, pg_no, offset);
            let mut chunk = Vec::<u8>::with_capacity(stride as usize);

            if pass == 1 {
                assert!(!reader.cache.get_mut().contains_key(&pg_no));
                reader.read(offset, &mut chunk)?;
                chunks.push(chunk);
            } else {
                // pg_no == 1 was deleted, because cache_size is 10 pages
                // and we read 11, so least recently used page (1) was deleted
                assert_eq!(reader.cache.get_mut().contains_key(&pg_no), pg_no != 1);
                reader.read(offset, &mut chunk)?;
                assert_eq!(chunk, chunks[pg_no as usize - 1]);
            }
        }
    }
    clean_db(&path);
    Ok(())
}

#[cfg(target_os = "windows")]
#[test]
pub fn caching_test_windows() -> Result<(), SimpleError> {
    let cache_size: usize = 10;
    let table = "test_table";
    let test_db = "caching_test.edb";
    println!("db {}", test_db);
    let path = prepare_db_gen(test_db, table, 1024 * 8, 1024, 1000);
    let mut reader = Reader::new(&path, cache_size as usize)?;
    let page_size = reader.page_size as u64;
    let num_of_pages = std::cmp::min(fs::metadata(&path).unwrap().len() / page_size, page_size) as usize;
    let full_cache_size = 6 * cache_size;
    let stride = num_of_pages / full_cache_size;
    let chunk_size = page_size as usize / num_of_pages;
    let mut chunks = Vec::<Vec<u8>>::with_capacity(stride as usize);

    println!("cache_size: {}, page_size: {}, num_of_pages: {}, stride: {}, chunk_size: {}",
        cache_size, page_size, num_of_pages, stride, chunk_size);

    for pass in 1..3 {
        for pg_no in 1_u32..12_u32 {
            let offset: u64 = pg_no as u64 * (page_size + chunk_size as u64);

            println!("pass {}, pg_no {}, offset {:x} ", pass, pg_no, offset);
            let mut chunk = Vec::<u8>::with_capacity(stride as usize);

            if pass == 1 {
                assert!(!reader.cache.get_mut().contains_key(&pg_no));
                reader.read(offset, &mut chunk)?;
                chunks.push(chunk);
            } else {
                // pg_no == 1 was deleted, because cache_size is 10 pages
                // and we read 11, so least recently used page (1) was deleted
                assert_eq!(reader.cache.get_mut().contains_key(&pg_no), pg_no != 1);
                reader.read(offset, &mut chunk)?;
                assert_eq!(chunk, chunks[pg_no as usize - 1]);
            }
        }
    }
    clean_db_gen(&path);
    Ok(())
}

fn check_row(jdb: &mut EseParser, table_id: u64, columns: &[ColumnInfo]) -> HashSet<String> {
    let mut values = HashSet::<String>::new();
    for col in columns {
        match jdb.get_column_str(table_id, col.id, col.cp) {
            Ok(result) => {
                if let Some(value) = result {
                    values.insert(value);
                } else {
                    values.insert("".to_string());
                }
			},
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

pub fn run_decompress_test(filename: &str, record_size : usize) -> Result<(), SimpleError> {
    let table = "test_table";
    let path = prepare_db(filename, table, 1024 * 8, record_size, 10);
    let mut jdb = EseParser::init(5);

    match jdb.load(path.to_str().unwrap()) {
        Some(e) => panic!("Error: {}", e),
        None => println!("Loaded {}", path.display())
    }

    let table_id = jdb.open_table(table)?;
    let columns = jdb.get_columns(table)?;

    assert!(jdb.move_row(table_id, ESE_MoveFirst));

    for i in 0.. {
		let values = check_row(&mut jdb, table_id, &columns);
		assert_eq!(values.len(), 1);
		let v = format!("Record {number:>width$}", number=i, width=record_size);
		assert!(values.contains(&v), "{}", true);
		if !jdb.move_row(table_id, ESE_MoveNext) {
			break;
		}
    }
    clean_db(&path);
    Ok(())
}
