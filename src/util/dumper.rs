//dumper.rs

pub use prettytable::{Table, Row, Cell};
extern crate hexdump;
use itertools::Itertools;
use crate::ese::db_file_header::{ esedb_file_header };

pub fn dump_db_file_header(db_file_header: esedb_file_header) {
    let mut table = Table::new();

    macro_rules! add_row {
        ($fld: expr, $val: expr) => {table.add_row(Row::new(vec![Cell::new($fld), Cell::new($val)]))}
    }

    macro_rules! add_field {
        ($fld: ident) => {add_row!(stringify!($fld), &db_file_header.$fld.to_string())}
    }
    macro_rules! add_dt_field {
        ($dt: ident) => {
            let s = format!("{:.4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}",
                            1900 + db_file_header.$dt[5] as u16, db_file_header.$dt[4], db_file_header.$dt[3],
                            db_file_header.$dt[2], db_file_header.$dt[1], db_file_header.$dt[0],);
            add_row!(stringify!($dt), &s);
        }
    }
    macro_rules! add_64_field {
        ($fld: ident) => {
            let s = u64::from_be_bytes(db_file_header.$fld).to_string();
            add_row!(stringify!($fld), &s);
        }
    }
    macro_rules! add_hex_field {
        ($fld: ident) => {
            let mut s: String = "".to_string();
            hexdump::hexdump_iter(&db_file_header.$fld).foreach(|line| { s.push_str(&line); s.push_str("\n"); } );
            add_row!(stringify!($fld), &s);
        }
    }

    add_field!(checksum);
    add_field!(signature);
    add_field!(format_version);
    add_field!(file_type);
    add_hex_field!(database_time);
    add_hex_field!(database_signature);
    add_field!(database_state);
    add_64_field!(consistent_postition);
    add_dt_field!(consistent_time);
    add_dt_field!(attach_time);
    add_64_field!(attach_postition);
    add_dt_field!(detach_time);
    add_64_field!(detach_postition);
    add_field!(unknown1);
    add_hex_field!(log_signature);
    add_hex_field!(previous_full_backup);
    add_hex_field!(previous_incremental_backup);
    add_hex_field!(current_full_backup);
    add_field!(shadowing_disabled);
    add_field!(last_object_identifier);
    add_field!(index_update_major_version);
    add_field!(index_update_minor_version);
    add_field!(index_update_build_number);
    add_field!(index_update_service_pack_number);
    add_field!(format_revision);
    add_field!(page_size);
    add_dt_field!(repair_time);
    add_hex_field!(unknown2);
    add_dt_field!(scrub_database_time);
    add_dt_field!(scrub_time);
    add_hex_field!(required_log);
    add_field!(upgrade_exchange5_format);
    add_field!(upgrade_free_pages);
    add_field!(upgrade_space_map_pages);
    add_hex_field!(current_shadow_volume_backup);
    add_field!(creation_format_version);
    add_field!(creation_format_revision);
    add_hex_field!(unknown3);
    add_field!(old_repair_count);
    add_field!(ecc_fix_success_count);
    add_dt_field!(ecc_fix_success_time);
    add_field!(old_ecc_fix_success_count);
    add_field!(ecc_fix_error_count);
    add_dt_field!(ecc_fix_error_time);
    add_field!(old_ecc_fix_error_count);
    add_field!(bad_checksum_error_count);
    add_dt_field!(bad_checksum_error_time);
    add_field!(old_bad_checksum_error_count);
    add_field!(committed_log);
    add_hex_field!(previous_shadow_volume_backup);
    add_hex_field!(previous_differential_backup);
    add_hex_field!(unknown4_1);
    add_hex_field!(unknown4_2);
    add_field!(nls_major_version);
    add_field!(nls_minor_version);
    add_hex_field!(unknown5_1);
    add_hex_field!(unknown5_2);
    add_hex_field!(unknown5_3);
    add_hex_field!(unknown5_4);
    add_hex_field!(unknown5_5);
    add_field!(unknown_flags);

    table.printstd();
}
