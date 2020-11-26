//config.rs
#![allow(deprecated)]
//use std::borrow::Cow;
use log::{ debug };
use clap::{ Arg, App };
use std::path::PathBuf;

pub struct Config {
    pub inp_file: PathBuf,
    pub report_file: PathBuf,
}

impl Config {
    pub fn new() -> Result<Config, String> {
        let matches = App::new("ESE DB dump")
            .version("0.1.0")
            .arg(Arg::with_name("in")
                .short("i")
                .long("input")
                .takes_value(true)
                .required(true)
                .help("Path to ESE db file"))
            .arg(Arg::with_name("out")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Path to output report"))
            .get_matches();

        let inp_file = matches.value_of("in").unwrap().to_owned();
        debug!(" inp_file: {}", inp_file);

        let report_file = matches.value_of("out").to_owned();
        match report_file {
            Some(s) => s,
            _ => &""
        };

        Config::new_for_file(&PathBuf::from(inp_file), &"")
    }


    pub fn new_from_env(env_key: &str) -> Result<Config, String> {
        let path = std::env::var(env_key);

        if let Ok(inp_file) = path {
            if !inp_file.is_empty() {
                return Config::new_for_file(&PathBuf::from(inp_file), &"");
            }
        }

        Err(format!("'{}' environment variable is not defined", env_key))
    }

    pub fn new_for_file(inp_file: &PathBuf, report_file: &str) -> Result<Config, String> {
        if inp_file.is_file() {
            return Ok(Config { inp_file: inp_file.canonicalize().unwrap(), report_file: PathBuf::from(report_file) });
        }

        Err(format!("{} is not a file", inp_file.display()))
    }
}
