//test_load_page_tags.rs

use crate::ese::jet;
use crate::tests::lib::*;
use std::io::{BufRead, Cursor};
use std::process::Command;

#[test]
fn test_load_page_tags() {
    let entourage = Entourage::new();
    let config = entourage.config;

    let io_handle = jet::IoHandle::new(&entourage.db_file_header);
    let pages = [
        jet::FixedPageNumber::Database,
        jet::FixedPageNumber::Catalog,
    ];

    for pg_no in pages.iter() {
        let db_page = jet::DbPage::new(&config, &io_handle, *pg_no as u32);
        println!("Page {:?}:", pg_no);
        //util::dumper::dump_page_header(&db_page.page_header);

        let mut tst_tags = vec![];
        {
            use nom::{
                bytes,
                bytes::complete::{is_not, tag},
                error,
                error::VerboseError,
                sequence::{separated_pair, terminated},
            };
            type Res<T, U> = nom::IResult<T, U, VerboseError<T>>;

            fn find_tag(line: &str) -> Res<&str, &str> {
                error::context("find_tag", tag("TAG "))(line)
            }

            fn find_data(line: &str) -> Res<&str, &str> {
                error::context("find_data", bytes::complete::is_not("c"))(line)
            }

            fn extract_data(line: &str) -> Res<&str, &str> {
                match find_data(line) {
                    Ok((data, _)) => terminated(is_not(" "), tag(" "))(data),
                    Err(e) => Err(e),
                }
            }

            fn get_hex_value(data: &str, val_name: &str) -> u32 {
                fn find<'a>(what: &'a str, line: &'a str) -> Res<&'a str, &'a str> {
                    tag(what)(line)
                }
                match find(val_name, data) {
                    Ok((val, _)) => u32::from_str_radix(val, 16).unwrap(),
                    _ => 0,
                }
            }

            #[derive(Debug)]
            struct TestTag {
                pub offset: u32,
                pub flags: u32,
                pub size: u32,
            }
            fn fill_tag(data: &str) -> TestTag {
                fn split(line: &str) -> Res<&str, (&str, &str)> {
                    separated_pair(is_not(","), tag(","), is_not(","))(line)
                }
                match split(data) {
                    Ok((_, (cb, ib))) => {
                        let size = get_hex_value(cb, "cb:0x");
                        let offset = get_hex_value(ib, "ib:0x");
                        TestTag {
                            size: size,
                            offset: offset,
                            flags: 0,
                        }
                    }
                    Err(e) => panic!("{:?}", e),
                }
            }

            let output = Command::new("esentutl")
                .args(&[
                    "/ms",
                    config.inp_file.to_str().unwrap(),
                    &format!("/p{}", *pg_no as u32),
                ])
                .output()
                .expect("failed to execute process");
            let file = Cursor::new(output.stdout);
            for line in file.lines() {
                match find_tag(&line.unwrap()[..]) {
                    Ok((res, _)) => {
                        let (_, data) = extract_data(&res).unwrap();
                        //println!("rem: '{}', data: '{}'", rem, data);
                        let tag = fill_tag(data);
                        println!("{:?}", tag);
                        tst_tags.push(tag);
                    }
                    _ => {}
                }
            }
        }

        let mut seq_no = 0;
        for pg_tag in &db_page.page_tags {
            let size = pg_tag.size();
            let offset = pg_tag.offset();
            println!(
                "offset: {:#x} ({}), size: {:#x} ({})",
                offset, offset, size, size
            );
            assert_eq!(tst_tags[seq_no].offset, offset);
            assert_eq!(tst_tags[seq_no].size, size);
            seq_no += 1;
        }
    }
}
