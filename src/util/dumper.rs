//dumper.rs

use prettytable::{Row, Cell};
use comfy_table;
extern crate hexdump;
use std::string::ToString;

use crate::ese::ese_db::{FileHeader, PageHeader};
use crate::ese::{jet, ese_db};
//use std::fmt::{self, /*Formatter,*/ Debug};

pub fn _dump_db_file_header(db_file_header: FileHeader) {
    let mut table = prettytable::Table::new();

    macro_rules! add_row {
        ($fld: expr, $val: expr) => {table.add_row(Row::new(vec![Cell::new($fld), Cell::new($val)]))}
    }

    macro_rules! add_field {
        ($fld: ident) => {
            let s = format!("{}", db_file_header.$fld);
            add_row!(stringify!($fld), &s)
        };
    }

    macro_rules! add_enum_field {
        ($fld: ident) => {
            let s = format!("{} ({})", db_file_header.$fld, db_file_header.$fld as u8);
            add_row!(stringify!($fld), &s)
        };
    }

    macro_rules! add_dt_field {
        ($fld: ident) => {
            add_row!(stringify!($fld), &db_file_header.$fld.to_string());
        }
    }

    macro_rules! add_db_time_field {
        ($fld: ident) => {
            add_row!(stringify!($fld), &db_file_header.$fld.to_string());
        }
    }

    macro_rules! add_bk_info_field {
        ($fld: ident) => {
            unsafe {
                let bk = db_file_header.$fld;
                let s = format!("Log Gen:{}-{} ({:#x}-{:#x}), Mark:{}", bk.gen_low, bk.gen_high, bk.gen_low, bk.gen_high, bk.bk_logtime_mark.to_string());
                add_row!(stringify!($fld), &s)
            }
        };
    }

    macro_rules! add_pos_field {
        ($fld: ident) => {
            unsafe {
                let v = &db_file_header.$fld;
                let s = format!("ib: {}, isec: {}, l_generation: {}", v.ib, v.isec, v.l_generation);
                add_row!(stringify!($fld), &s);
            }
        }
    }

    macro_rules! add_hex_field {
        ($fld: ident) => {
            let mut s: String = "".to_string();
            hexdump::hexdump_iter(&db_file_header.$fld).for_each(|line| { s.push_str(&line); s.push_str("\n"); } );
            add_row!(stringify!($fld), &s);
        }
    }
    macro_rules! add_sign_field {
        ($fld: ident) => {
            let sign = &db_file_header.$fld;
            let dt = &sign.logtime_create;
            let s = format!("Create time: {} Rand:{} Computer: {}",
                                dt.to_string(), sign.random, std::str::from_utf8(&sign.computer_name).unwrap());
            add_row!(stringify!($fld), &s);
        }
    }

    add_field!(checksum);
    add_field!(signature);
    add_field!(format_version);
    add_field!(file_type);
    add_db_time_field!(database_time);
    add_sign_field!(database_signature);
    add_enum_field!(database_state);
    add_pos_field!(consistent_postition);
    add_dt_field!(consistent_time);
    add_dt_field!(attach_time);
    add_pos_field!(attach_postition);
    add_dt_field!(detach_time);
    add_pos_field!(detach_postition);
    add_field!(dbid);
    add_sign_field!(log_signature);
    add_bk_info_field!(previous_full_backup);
    add_bk_info_field!(previous_incremental_backup);
    add_bk_info_field!(current_full_backup);
    add_field!(shadowing_disabled);
    add_field!(last_object_identifier);
    add_field!(index_update_major_version);
    add_field!(index_update_minor_version);
    add_field!(index_update_build_number);
    add_field!(index_update_service_pack_number);

    //add_field!(format_revision);
    add_row!("format_revision", &jet::revision_to_string(db_file_header.format_version, db_file_header.format_revision));

    add_field!(page_size);
    add_dt_field!(repair_time);
    add_field!(repair_count);
    add_dt_field!(scrub_database_time);
    add_dt_field!(scrub_time);
    add_hex_field!(required_log);
    add_field!(upgrade_exchange5_format);
    add_field!(upgrade_free_pages);
    add_field!(upgrade_space_map_pages);
    add_bk_info_field!(current_shadow_volume_backup);
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
    add_bk_info_field!(previous_shadow_volume_backup);
    add_bk_info_field!(previous_differential_backup);
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

pub fn dump_page_header(page_header: &ese_db::PageHeader) {
    let table = comfy_table::Table::new();

    // macro_rules! add_row {
    //     ($fld: expr, $val: expr) => {
    //         table.add_row(vec![comfy_table::Cell::new($fld), comfy_table::Cell::new($val)])
    //     }
    // }

    match page_header {
        PageHeader::old (pg_hdr, pg_common) => {
            println!("{:?} {:?}", pg_hdr, pg_common)
        },
        PageHeader::x0b(pg_hdr, pg_common) => {
            println!("{:?} {:?}", pg_hdr, pg_common)
        },
        PageHeader::x11(pg_hdr, pg_common) => {
            println!("{:?} {:?}", pg_hdr, pg_common)
        },
        PageHeader::x11_ext(pg_hdr, pg_common, pg_ext) => {
            println!("{:?} {:?} {:?}", pg_hdr, pg_common, pg_ext)
        },
    }

    println!("{}", table);

}

/*impl Debug for jet::PageHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.page_header {
            page_header::old(page) => write!(f, "{:?}", page),
            page_header::x0b(page) => write!(f, "{:?}", page),
            page_header::x11(page, ..) => write!(f, "{:?}", page),
        }
    }
}

 */

