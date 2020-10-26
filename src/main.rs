#![feature(maybe_uninit_ref)]
#![allow(non_camel_case_types,  clippy::mut_from_ref, clippy::cast_ptr_alignment)]
#[macro_use] extern crate log;
extern crate strum;

use env_logger;

use ese_parser::util::config::Config;
use ese_parser::util::reader::load_db_file_header;

use std::process;

fn main() {
    env_logger::init();

    let config = Config::new().unwrap_or_else(|err| {  error!("Problem parsing arguments: {}", err);
                                                                   process::exit(1);
                                                                });
    info!("file '{}'", config.inp_file.display());

    let db_file_header = match load_db_file_header(&config) {
        Ok(x) => x ,
        Err(e) => {
            error!("Application error: {}", e);
            process::exit(1);
        }
    };

    use ese_parser::util::dumper::dump_db_file_header;
    dump_db_file_header(db_file_header);
}
