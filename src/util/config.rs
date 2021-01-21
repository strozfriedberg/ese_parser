//config.rs
#![allow(deprecated)]
use clap::{App, Arg};
use log::debug;
use std::borrow::Cow;
use std::path::PathBuf;

pub struct Config {
    pub inp_file: PathBuf,
    pub report_file: PathBuf,
}

impl Config {
    //due "warning: associated function is never used: `new`" while main.rs:43
    #[allow(dead_code)]
    pub fn new() -> Result<Config, Cow<'static, str>> {
        let matches = App::new("ESE DB dump")
            .version("0.1.0")
            .arg(
                Arg::with_name("in")
                    .short("i")
                    .long("input")
                    .takes_value(true)
                    .required(true)
                    .help("Path to ESE db file"),
            )
            .arg(
                Arg::with_name("out")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .help("Path to output report"),
            )
            .get_matches();

        let inp_file = matches.value_of("in").unwrap().to_owned();
        debug!(" inp_file: {}", inp_file);

        let report_file = matches.value_of("out").to_owned();
        match report_file {
            Some(s) => s,
            _ => &"",
        };

        Config::new_for_file(&PathBuf::from(inp_file), &"")
    }

    pub fn _new_from_env(env_key: &str) -> Result<Config, Cow<'static, str>> {
        let path = std::env::var(env_key);

        if let Ok(inp_file) = path {
            if !inp_file.is_empty() {
                return Config::new_for_file(&PathBuf::from(inp_file), &"");
            }
        }

        Err(format!("'{}' environment variable is not defined", env_key).into())
    }

    //due "warning: associated function is never used: `new_for_file`" while config.rs:41 (unconditionally)
    #[allow(dead_code)]
    pub fn new_for_file(
        inp_file: &PathBuf,
        report_file: &str,
    ) -> Result<Config, Cow<'static, str>> {
        if inp_file.is_file() {
            return Ok(Config {
                inp_file: inp_file.canonicalize().unwrap(),
                report_file: PathBuf::from(report_file),
            });
        }

        Err(format!("{} is not a file", inp_file.display()).into())
    }
}
