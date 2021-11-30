#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types, clippy::mut_from_ref, clippy::cast_ptr_alignment, clippy::approx_constant)]
mod parser;

#[cfg(target_os = "windows")]
pub mod esent;

pub mod ese_trait;
pub mod ese_parser;
pub mod vartime;
pub mod process_tables;

use ese_trait::*;
#[cfg(test)]
fn init_tests(cache_size: usize, db: Option<&str>) -> ese_parser::EseParser{
    use ese_trait::*;
    let mut jdb = ese_parser::EseParser::init(cache_size);
    match jdb.load(&["testdata", db.unwrap_or("test.edb")].join("/")) {
        Some(e) => panic!("Error: {}", e),
        None => println!("Loaded.")
        }
    jdb
}

fn check_table_names(expected_tables:Vec<&str>, jdb: ese_parser::EseParser) {
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

#[test]
fn test_columns() {
    let jdb = init_tests(5, None);
    let table = "TestTable";
    println!("table {}", table);
    let columns = jdb.get_columns(table).unwrap();

    let table_id = jdb.open_table(table).unwrap();
    assert!(jdb.move_row(table_id, ESE_MoveFirst), "{}", true);

    let auto_inc = columns.iter().find(|x| x.name == "AutoInc" ).unwrap();
    assert_eq!(jdb.get_fixed_column::<i8>(table_id, auto_inc.id).unwrap(), Some(1));

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
        let s = lt.as_slice();

        let mut v16: Vec<u16> = vec![0;s.len()/std::mem::size_of::<u16>()];
        LittleEndian::read_u16_into(&s, &mut v16);
        let ws = widestring::U16String::from_vec(v16);
        for i in 0..ws.len() {
            let l = ws.chars().nth(i).unwrap().unwrap();
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


#[test]
fn test_something(){

}