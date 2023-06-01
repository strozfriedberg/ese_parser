use std::fs::File;
use std::io::BufReader;

use ese_parser_lib::ese_parser::EseParser;
use ese_parser_lib::parser::reader;
use ese_parser_lib::parser::reader::Reader;
use ese_parser_lib::parser::win::gen_db::prepare_db_gen;

use bitvec::vec::BitVec;

fn main() {
    let cache_size: usize = 10;
    let table = "test_table";
    let test_db = "caching_test.edb";
    println!("db {}", test_db);

    let path = prepare_db_gen(test_db, table, 1024 * 8, 1024, 10);

    let db = EseParser::load_from_path(cache_size, path);

    let used: &BitVec = reader::get_used();
    let zeros = used.iter_zeros();
    println!("{:?}", zeros.take(100));
}