
#[cfg(test)]
mod tests {
    use md5;
    use std::path::PathBuf;
    use std::fs;
    use crate::process_tables::*;
    use std::string::String;

    fn md5_digest(input: Vec<u8>) -> String {
        let digest = md5::compute(input);
        format!("{:x}",digest)
    }

    fn get_file_contents(file_path: &PathBuf, db_path: &PathBuf) -> Vec<u8> {
        process_table(db_path.to_str().unwrap(), Some(file_path.clone()), Mode::EseParser, String::new());
        let contents = fs::read(file_path).unwrap();
        contents
    }

    #[test]
    fn test_compare_output() {
        let mut esent_file_path = PathBuf::from("testdata").canonicalize().unwrap();
        esent_file_path.push("esentoutput2.txt");
        let mut parser_output_path = PathBuf::from("testdata").canonicalize().unwrap();
        parser_output_path.push("parseroutput.txt");
        let mut db_path = PathBuf::from("testdata").canonicalize().unwrap();
        db_path.push("decompress_test.edb");

        let parser_file = get_file_contents(&parser_output_path, &db_path);
        let esent_file = fs::read(esent_file_path).unwrap();

        let esent_digest = md5_digest(esent_file);
        let parser_digest = md5_digest(parser_file);

        assert_eq!(esent_digest, parser_digest);
        std::fs::remove_file(parser_output_path).unwrap();
    }
}