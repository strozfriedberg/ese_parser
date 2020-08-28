use std::error::Error;
use std::process;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;

pub struct Config {
    pub filename: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }

        let filename = args[1].clone();

        Ok(Config { filename })
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
    let contents = get_file_as_byte_vec(&config.filename);

    println!("{:0x?}", &contents[..128]);

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
                                                                            println!("Problem parsing arguments: {}", err);
                                                                            process::exit(1);
                                                                        });

    println!("file '{}'", config.filename);

    if let Err(e) = run(config) {
        println!("Application error: {}", e);

        process::exit(1);
    }
}
