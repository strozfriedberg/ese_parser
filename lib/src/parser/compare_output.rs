
#[cfg(test)]
mod tests {
    use md5;
    use std::path::PathBuf;
    use std::fs;
    use crate::process_tables::*;
    use std::string::String;
    use regex::RegexSet;

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

    #[test]
    fn test_datetimes() {
        let mut parser_output_path = PathBuf::from("testdata").canonicalize().unwrap();
        parser_output_path.push("KStrike_UAL");
        parser_output_path.push("parseroutput.txt");
        let parser_output_path_clone = parser_output_path.clone();
        let mut db_path = PathBuf::from("testdata").canonicalize().unwrap();
        db_path.push("KStrike_UAL");
        db_path.push("Current.mdb");

        get_file_contents(&parser_output_path, &db_path);
        let parser_output = fs::read_to_string(&parser_output_path).unwrap();
        std::fs::remove_file(parser_output_path_clone).unwrap();

        let set = RegexSet::new(&[
            r"2021-06-12 23:47:21.232323500",
            r"2021-06-12 23:48:45.468902200",
            r"2077-06-12 23:48:45.468902200", //no match, so in [0, 1, 3] 2 is missing in the assert below
            r"2021-06-20 20:48:48.366866900",
        ]).unwrap();
        let matches: Vec<_> = set.matches(&parser_output).into_iter().collect();
        assert_eq!(matches, vec![0, 1, 3]);
    }
}