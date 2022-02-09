#![allow(
    non_upper_case_globals,
    non_snake_case,
    non_camel_case_types,
    clippy::mut_from_ref,
    clippy::cast_ptr_alignment
)]

mod process_tables;
mod compare_output;

use crate::process_tables::*;
use std::env;

fn main() {
    let mut table = String::new();
    let mut mode: Mode = {
        #[cfg(target_os = "windows")]
        {
            Mode::Both
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            Mode::EseParser
        }
    };
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("db path required");
        return;
    }
    if args[0].contains("help") {
        eprintln!("[/m mode] [/t table] db path");
        eprintln!("where mode one of [EseAPI, EseParser, *Both - default]");
        std::process::exit(0);
    }
    if args[0].to_lowercase() == "/m" {
        if args[1].to_lowercase() == "eseapi" {
            mode = Mode::EseApi;
        } else if args[1].to_lowercase() == "eseparser" {
            mode = Mode::EseParser;
        } else if args[1].to_lowercase() == "both" {
            mode = Mode::Both;
        } else {
            eprintln!("unknown mode: {}", args[1]);
            std::process::exit(-1);
        }
        args.drain(..2);
    }
    if args[0].to_lowercase() == "/t" {
        table = args[1].clone();
        args.drain(..2);
    }
    if args.is_empty() {
        eprintln!("db path required");
        std::process::exit(-1);
    }
    let dbpath = args.concat();

    process_table(&dbpath, None, mode, table);
}
