#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types, clippy::mut_from_ref, clippy::cast_ptr_alignment, clippy::approx_constant)]
pub mod parser;

#[cfg(all(feature = "nt_comparison", target_os = "windows"))]
pub mod esent;

pub mod ese_trait;
pub mod ese_parser;
pub mod vartime;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;
    use super::ese_trait::*;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;

    fn init_tests(cache_size: usize, db: Option<&str>) -> ese_parser::EseParser<BufReader<File>> {
        let path = &["testdata", db.unwrap_or("test.edb")].join("/");
        ese_parser::EseParser::load_from_path(cache_size, path).unwrap();
        match ese_parser::EseParser::load_from_path(cache_size, path) {
            Ok(jdb) => jdb,
            Err(e) => panic!("Error: {}", e)
        }
    }

    #[cfg(test)]
    fn check_table_names(expected_tables:Vec<&str>, jdb: ese_parser::EseParser<BufReader<File>>) {
        let tables = jdb.get_tables().unwrap();
        assert_eq!(tables.len(), expected_tables.len());
        for i in 0..tables.len() {
            assert_eq!(tables[i], expected_tables[i]);
        }
    }

    #[test]
    fn test_edb_table_default() {
        let jdb = init_tests(5, None); // None means default db is used: test.db

        let expected_tables = vec!["MSysObjects", "MSysObjectsShadow", "MSysObjids", "MSysLocales", "TestTable"];
        check_table_names(expected_tables, jdb);
    }

    #[test]
    fn test_edb_table_decompress() {
        let jdb = init_tests(5, Some("decompress_test.edb")); // None means default db is used: test.db

        let expected_tables = vec!["MSysObjects", "MSysObjectsShadow", "MSysObjids", "MSysLocales", "test_table"];
        check_table_names(expected_tables, jdb);
    }

    #[cfg(test)]
    fn check_datetimes(db: &str) {
        let jdb = init_tests(5, Some(db));
        let columns = jdb.get_columns("CLIENTS").unwrap();
        let table_id = jdb.open_table("CLIENTS").unwrap();
        let insert_date = columns.iter().find(|x| x.name == "InsertDate" ).unwrap();
        let dates = vec!["2021-06-12 23:47:21.232323500 UTC",
                        "2021-06-12 23:48:45.468902200 UTC"];
        for expected_datetime in dates.into_iter() {
            let column_contents = jdb.get_column_date(table_id, insert_date.id).unwrap().unwrap();
            assert_eq!(column_contents.format("%Y-%m-%d %H:%M:%S.%f %Z").to_string(), expected_datetime.to_string());
            jdb.move_row(table_id, ESE_MoveNext);
        }
    }

    #[test]
    fn test_datetime_current(){
        check_datetimes("Current.mdb");
    }

    #[test]
    fn test_datetime_guid(){
        //expect same dates because current and GUID are from year 2021
        check_datetimes("{03A01CC5-91BB-4936-B685-63697785D39E}.mdb");
    }

    #[cfg(test)]
    fn get_column_names(columns: Vec<ColumnInfo>) -> Vec<String> {
        let mut column_names= vec!();
        for ci in columns.iter() {
            column_names.push(ci.name.clone());
        }
        column_names
    }

    #[test]
    fn test_system_identity_columns() {
        let jdb = init_tests(5, Some("SystemIdentity.mdb"));
        let mut tables_map = HashMap::new();
        tables_map.insert(String::from("SYSTEM_IDENTITY"), vec!("CreationTime", "PhysicalProcessorCount", "CoresPerPhysicalProcessor", "LogicalProcessorsPerPhysicalProcessor", "MaximumMemory", "OSMajor", "OSMinor", "OSBuildNumber", "OSPlatformId", "ServicePackMajor", "ServicePackMinor", "OSSuiteMask", "OSProductType", "OSCurrentTimeZone", "OSDaylightInEffect", "SystemManufacturer", "SystemProductName", "SystemSMBIOSUUID", "SystemSerialNumber", "SystemDNSHostName", "SystemDomainName", "OSSerialNumber", "OSCountryCode", "OSLastBootUpTime"));
        tables_map.insert(String::from("CHAINED_DATABASES"), vec!("Year", "FileName"));
        tables_map.insert(String::from("ROLE_IDS"), vec!("RoleGuid", "ProductName", "RoleName"));
        for (table_name, coulumn_names) in &tables_map {
            let columns = jdb.get_columns(table_name).unwrap();
            assert_eq!(&get_column_names(columns), coulumn_names);
        }
    }

    #[cfg(test)]
    fn get_str_value(db_name: &str, table_name: &str, column_name: &str) -> String {
        let jdb = init_tests(5, Some(db_name));
        let columns = jdb.get_columns(table_name).unwrap();
        let table_id = jdb.open_table(table_name).unwrap();
        let column_info = columns.iter().find(|x| x.name == column_name).unwrap();
        jdb.get_column_str(table_id, column_info.id, column_info.cp).unwrap().unwrap()
    }

    #[test]
    fn test_system_identity_values() {
        let column_contents = get_str_value("SystemIdentity.mdb","CHAINED_DATABASES", "FileName");
        assert_eq!(column_contents, "{03A01CC5-91BB-4936-B685-63697785D39E}.mdb\u{0}");
        let dns_host_name = get_str_value("SystemIdentity.mdb","SYSTEM_IDENTITY", "SystemDNSHostName");
        assert_eq!(dns_host_name, "WIN-M5M48LSM8UB\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}");
    }

    #[test]
    fn test_vartime_datetime() {
        let jdb = init_tests(5, Some("test.edb"));
        let columns = jdb.get_columns("TestTable").unwrap();
        let table_id = jdb.open_table("TestTable").unwrap();
        let insert_date = columns.iter().find(|x| x.name == "DateTime" ).unwrap();
        let expected_datetime = "2021-03-29 11:49:47.000000000 UTC";
        let column_contents = jdb.get_column_date(table_id, insert_date.id).unwrap().unwrap();
        assert_eq!(column_contents.format("%Y-%m-%d %H:%M:%S.%f %Z").to_string(), expected_datetime.to_string());
    }

    #[test]
    fn test_columns() {
        let jdb = init_tests(5, None);
        let table = "TestTable";
        println!("table {}", table);
        let columns = jdb.get_columns(table).unwrap();

        let table_id = jdb.open_table(table).unwrap();
        assert!(jdb.move_row(table_id, ESE_MoveFirst), "{}", true);

        let bit = columns.iter().find(|x| x.name == "Bit" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<i8>(table_id, bit.id).unwrap(), Some(0));

        let unsigned_byte = columns.iter().find(|x| x.name == "UnsignedByte" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<u8>(table_id, unsigned_byte.id).unwrap(), Some(255));

        let short = columns.iter().find(|x| x.name == "Short" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<i16>(table_id, short.id).unwrap(), None);

        let long = columns.iter().find(|x| x.name == "Long" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<i32>(table_id, long.id).unwrap(), Some(-2147483648));

        let currency = columns.iter().find(|x| x.name == "Currency" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<i64>(table_id, currency.id).unwrap(), Some(350050)); // 35.0050

        let float = columns.iter().find(|x| x.name == "IEEESingle" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<f32>(table_id, float.id).unwrap(), Some(3.141592));

        let double = columns.iter().find(|x| x.name == "IEEEDouble" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<f64>(table_id, double.id).unwrap(), Some(3.141592653589));

        let unsigned_long = columns.iter().find(|x| x.name == "UnsignedLong" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<u32>(table_id, unsigned_long.id).unwrap(), Some(4294967295));

        let long_long = columns.iter().find(|x| x.name == "LongLong" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<i64>(table_id, long_long.id).unwrap(), Some(9223372036854775807));

        let unsigned_short = columns.iter().find(|x| x.name == "UnsignedShort" ).unwrap();
        assert_eq!(jdb.get_fixed_column::<u16>(table_id, unsigned_short.id).unwrap(), Some(65535));

        // DateTime
        {
            let date_time = columns.iter().find(|x| x.name == "DateTime" ).unwrap();
            let dt = jdb.get_fixed_column::<f64>(table_id, date_time.id).unwrap().unwrap();

            let mut st = unsafe { std::mem::MaybeUninit::<vartime::SYSTEMTIME>::zeroed().assume_init() };
            let r = vartime::VariantTimeToSystemTime(dt, &mut st);
            assert!(r, "{}", true);
            assert_eq!(st.wDayOfWeek, 1);
            assert_eq!(st.wDay, 29);
            assert_eq!(st.wMonth, 3);
            assert_eq!(st.wYear, 2021);
            assert_eq!(st.wHour, 11);
            assert_eq!(st.wMinute, 49);
            assert_eq!(st.wSecond, 47);
            assert_eq!(st.wMilliseconds, 0);
        }

        // GUID
        {
            let guid = columns.iter().find(|x| x.name == "GUID" ).unwrap();
            let v = jdb.get_column(table_id, guid.id).unwrap().unwrap();
            let s = format!("{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
                v[3], v[2], v[1], v[0], v[5], v[4], v[7], v[6], v[8], v[9], v[10], v[11], v[12], v[13], v[14], v[15]);
            assert_eq!(s, "{4D36E96E-E325-11CE-BFC1-08002BE10318}");
        }

        // Binary
        {
            let binary = columns.iter().find(|x| x.name == "Binary" ).unwrap();
            assert_eq!(binary.cbmax, 255);

            let b = jdb.get_column(table_id, binary.id).unwrap().unwrap();
            for (i,&bin) in b.iter().enumerate() {
                assert_eq!(bin, (i % 255) as u8);
            }
        }

        // LongBinary
        {
            let long_binary = columns.iter().find(|x| x.name == "LongBinary" ).unwrap();
            assert_eq!(long_binary.cbmax, 65536);

            let b = jdb.get_column(table_id, long_binary.id).unwrap().unwrap();
            assert_eq!(b.len(), 128);
            for (i,&long_bin) in b.iter().enumerate() {
                assert_eq!(long_bin, (i % 255) as u8);
            }

            // multi-test value inside of long-value test
            let b = jdb.get_column_mv(table_id, long_binary.id, 2).unwrap().unwrap();
            assert_eq!(b.len(), 65536);
            for (i,&long_bin) in b.iter().enumerate() {
                assert_eq!(long_bin, (i % 255) as u8);
            }
        }

        let abc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890";

        // Text
        {
            let text = columns.iter().find(|x| x.name == "Text" ).unwrap();
            assert_eq!(text.cbmax, 255);
            assert_eq!(text.cp, ESE_CP::ASCII as u16);

            let str = jdb.get_column_str(table_id, text.id, text.cp).unwrap().unwrap();
            let t = jdb.get_column(table_id, text.id).unwrap().unwrap();
            for (i,&text) in t.iter().enumerate() {
                let expected_char = abc.as_bytes()[i % abc.len()];
                assert_eq!(text, expected_char);
                assert_eq!(str.as_bytes()[i], expected_char);
            }

            // Multi-value test
            let v = jdb.get_column_mv(table_id, text.id, 2).unwrap().unwrap();
            let h = "Hello".to_string();
            assert_eq!(v.len()-2, h.len());
            for (i, &text) in v.iter().enumerate().take(v.len()-2) {
                assert_eq!(text, h.as_bytes()[i]);
            }
        }

        // LongText (compressed)
        {
            let long_text = columns.iter().find(|x| x.name == "LongText" ).unwrap();
            assert_eq!(long_text.cbmax, 8600);
            assert_eq!(long_text.cp, ESE_CP::Unicode as u16);

            let lt = jdb.get_column(table_id, long_text.id).unwrap().unwrap();
            let ws = crate::utils::from_utf16(&lt).unwrap();
            for i in 0..ws.len() {
                let l = ws.chars().nth(i).unwrap();
                let r = abc.as_bytes()[i % abc.len()] as char;
                assert_eq!(l, r);
            }
        }

        // Default value
        {
            let deftext = columns.iter().find(|x| x.name == "TextDefaultValue" ).unwrap();
            assert_eq!(deftext.cbmax, 255);
            assert_eq!(deftext.cp, ESE_CP::ASCII as u16);
            let str = jdb.get_column_str(table_id, deftext.id, deftext.cp).unwrap().unwrap();
            let defval = "Default value.".to_string();
            assert_eq!(str.len()-1, defval.len());
            for i in 0..str.len()-1 {
                assert_eq!(str.as_bytes()[i], defval.as_bytes()[i]);
            }
        }

        jdb.close_table(table_id);
    }
}