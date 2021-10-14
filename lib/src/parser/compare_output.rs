//#![cfg(test)]

use md5;
use std::path::PathBuf;
use std::fs;

// #[cfg(target_os = "windows")]
// use crate::parser::reader::gen_db::*;

// pub fn get_parser {


// }



fn md5_digest(input: String) -> String {
    let digest = md5::compute(input);
    format!("{:x}",digest)
}

fn get_file_contents(filename: &str) -> String {
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