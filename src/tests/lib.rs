//lib

use std::path::PathBuf;

use crate::ese::ese_db::FileHeader;
use crate::util::config::Config;
use crate::util::reader::load_db_file_header;

pub(crate) const TEST_FILE: &str = r"data\test.edb";

pub(crate) struct Entourage {
    pub(crate) config: Config,
    pub(crate) db_file_header: FileHeader,
}

impl Entourage {
    pub fn new() -> Entourage {
        let _ = env_logger::try_init().or::<()>(Ok(()));

        let config = match Config::new_for_file(&PathBuf::from(TEST_FILE), &"") {
            Ok(x) => x,
            Err(e) => panic!("Could not create config: {}", e),
        };

        let db_file_header = match load_db_file_header(&config) {
            Ok(x) => x,
            Err(e) => panic!("Application error: {}", e),
        };

        Entourage {
            config: config,
            db_file_header: db_file_header,
        }
    }
}
