#![cfg(test)]

use md5;
use std::path::PathBuf;
use std::fs;
use crate::process_tables::*;

pub fn get_path(filename: &str) -> PathBuf {
    let mut dst_path = PathBuf::from("testdata").canonicalize().unwrap();
    dst_path.push(filename);
    dst_path
}

    // let mut f = OpenOptions::new()
    //     .read(true)
    //     .write(true)
    //     .open("parseroutput.txt")
    //     .unwrap();
    
    // //let path = f.path().unwrap();
    // process_table(dbpath);
    // dst_path

fn md5_digest(input: Vec<u8>) -> String {
    let digest = md5::compute(input);
    format!("{:x}",digest)
}

fn get_file_contents(filename: &str, dbname: &str) -> Vec<u8> {
    let mut file_path = PathBuf::from("testdata").canonicalize().unwrap();
    let mut db_path = PathBuf::from("testdata").canonicalize().unwrap();
    file_path.push(filename);
    //let test_file = File::open(file_path).unwrap();
    db_path.push(dbname);
    process_table(db_path.to_str().unwrap(), Some(file_path));
    
    let contents = fs::read(file_path).unwrap();
    contents
}

#[test]
fn test_compare_output() {
    
    let esent_file = "esentoutput.txt"; //should remain static
    let esent_file_path = get_path(esent_file);
    let parser_file = "parseroutput.txt";
    let parser_file_path = get_path(parser_file);

    let esent_file_contents = fs::read_to_string(esent_file_path).unwrap();
    let parser_file_contents = fs::read_to_string(parser_file_path).unwrap(); 

    let esent_digest = md5_digest(esent_file_contents.into_bytes());
    let parser_digest = md5_digest(parser_file_contents.into_bytes());

    assert_eq!(esent_digest, parser_digest);
}