//#![cfg(test)]

use md5;
use std::path::PathBuf;
use std::fs;
use crate::process_tables::*;
use std::process::Command;
use std::fs::File;
use std::io::{self, Error, Write};
use filepath::FilePath;
use std::env;
use std::fs::OpenOptions;

// #[cfg(target_os = "windows")]
// use crate::parser::reader::gen_db::*;


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

fn get_file_contents(filename: &str) -> Vec<u8> {
    let mut dst_path = PathBuf::from("testdata").canonicalize().unwrap();
    dst_path.push(filename);
    let contents = fs::read(dst_path).unwrap();
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