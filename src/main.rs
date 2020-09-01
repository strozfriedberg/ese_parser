use std::error::Error;
use std::process;
use std::fs;
use std::fs::File;
use std::io::Read;

extern crate clap;
use clap::{Arg, App};

pub struct Config {
    pub inp_file: String,
    pub report_file: Option<&'static str>,
}

impl Config {
    pub fn new() -> Result<Config, &'static str> {
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

        let inp_file = matches.value_of("in").unwrap();
        println!(" inp_file: {}", inp_file);

        let report_file = matches.value_of("out");
        match report_file {
            None => println!("No idea what your favorite number is."),
            Some(s) => {
                match s.parse::<i32>() {
                    Ok(n) => println!("Your favorite number must be {}.", n + 5),
                    Err(_) => println!("That's not a number! {}", s),
                }
            }
        }

        Ok(Config { inp_file : inp_file.to_string(), report_file : report_file.clone() })
    }
}

fn get_file_as_byte_vec(filename: &String) -> Vec<u8> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = get_file_as_byte_vec(&config.inp_file);

    println!("{:0x?}", &contents[..128]);

    Ok(())
}

fn main() {
    let config = Config::new().unwrap_or_else(|err| {  println!("Problem parsing arguments: {}", err);
                                                                   process::exit(1);
                                                                });

    println!("file '{}'", config.inp_file);

    if let Err(e) = run(config) {
        println!("Application error: {}", e);

        process::exit(1);
    }
}
