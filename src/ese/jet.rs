//jet.rs
#![allow( non_camel_case_types, dead_code )]

use std::{mem, fmt};
use chrono::TimeZone;
use chrono::naive::{NaiveDate, NaiveTime};
use winapi::um::timezoneapi::{GetTimeZoneInformation/*, FileTimeToSystemTime*/};
use strum::Display;

pub type uint8_t = ::std::os::raw::c_uchar;
pub type uint16_t = ::std::os::raw::c_short;
pub type uint32_t = ::std::os::raw::c_ulong;
pub type uint64_t = ::std::os::raw::c_ulonglong;

type OsDateTime = chrono::DateTime<chrono::Utc>;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct DbTime {
    pub hours: uint16_t,
    pub minutes: uint16_t,
    pub seconds: uint16_t,
    pub padding: uint16_t,
}

impl fmt::Display for DbTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(t) = NaiveTime::from_hms_opt(self.hours as u32, self.minutes as u32, self.seconds as u32) {
            write!(f, "{}", t)
        }
        else {
            write!(f, "Bad DbTime: {:?}", self)
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct DateTime {
    pub seconds: uint8_t,
    pub minutes: uint8_t,
    pub hours: uint8_t,
    pub day: uint8_t,
    pub month: uint8_t,
    pub year: uint8_t,
    pub time_is_utc: uint8_t,
    pub os_snapshot: uint8_t,
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.year > 0 {
            let ndt = NaiveDate::from_ymd(self.year as i32 + 1900, self.month as u32, self.day as u32)
                .and_hms(self.hours as u32, self.minutes as u32, self.seconds as u32);
            let offset = if self.time_is_utc != 0 {
                0
            }
            else {
                unsafe{
                    let mut tz = mem::zeroed();
                    GetTimeZoneInformation(&mut tz);
                    -60 * (tz.Bias + tz.StandardBias)
                }
            };
            let dt: OsDateTime = OsDateTime::from(chrono::FixedOffset::east(offset).from_local_datetime(&ndt).unwrap());

            write!(f, "{}", dt)
        } else {
            write!(f, "")
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Signature {
    pub random: uint32_t,
    pub logtime_create: DateTime,
    pub computer_name: [uint8_t; 16],
}

#[derive(Copy, Clone, Display, Debug)]
#[repr(u32)]
pub enum DbState {
    JustCreated = 1,
    DirtyShutdown = 2,
    CleanShutdown = 3,
    BeingConverted =4,
    ForceDetach = 5
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct LgPos {
    pub ib: ::std::os::raw::c_ushort,
    pub isec: ::std::os::raw::c_ushort,
    pub l_generation: ::std::os::raw::c_long,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct BackupInfo {
    pub lg_pos_mark: LgPos,
    pub bk_logtime_mark: DateTime,
    pub gen_low: ::std::os::raw::c_ulong,
    pub gen_high: ::std::os::raw::c_ulong,
}

pub fn revision_to_string(version: uint32_t, revision: uint32_t) -> String {
    let s = match (version, revision) {
                    (0x00000620, 0x00000000) => "Original operating system Beta format (April 22, 1997)",
                    (0x00000620, 0x00000001) => "Add columns in the catalog for conditional indexing and OLD (May 29, 1997)",
                    (0x00000620, 0x00000002) => "Add the fLocalizedText flag in IDB (July 5, 1997), Revert revision in order for ESE97 to remain forward-compatible (January 28, 1998)",
                    (0x00000620, 0x00000003) => "Add SPLIT_BUFFER to space tree root pages (October 30, 1997), Add new tagged columns to catalog (\"CallbackData\" and \"CallbackDependencies\")",
                    (0x00000620, 0x00000004) => "Super Long Value (SLV) support: signSLV, fSLVExists in db header (May 5, 1998)",
                    (0x00000620, 0x00000005) => "New SLV space tree (May 29, 1998)",
                    (0x00000620, 0x00000006) => "SLV space map (October 12, 1998)",
                    (0x00000620, 0x00000007) => "4-byte IDXSEG (December 10, 1998)",
                    (0x00000620, 0x00000008) => "New template column format (January 25, 1999)",
                    (0x00000620, 0x00000009) => "Sorted template columns (July 24, 1999). Used in Windows XP SP3",
                    (0x00000620, 0x0000000b) => "Contains the page header with the ECC checksum Used in Exchange",
                    (0x00000620, 0x0000000c) => "Used in Windows Vista (SP0)",
                    (0x00000620, 0x00000011) => "Support for 2 KiB, 16 KiB and 32 KiB pages. Extended page header with additional ECC checksums. Column compression. Space hints. Used in Windows 7 (SP0)",
                    (0x00000620, 0x00000014) => "Used in Exchange 2013 and Active Directory 2016",
                    (0x00000623, 0x00000000) => "New Space Manager (May 15, 1999)",
                    _ => "Unknown",
                };
    format!("{:#x}, {:#x}: {}", version, revision, s)
}

/*
use winapi::um::minwinbase::{SYSTEMTIME, LPSYSTEMTIME};
use winapi::shared::minwindef::FILETIME;
pub type FileTime = FILETIME;

pub fn filetime_to_string(ft: FileTime) -> String {
    let mut st: SYSTEMTIME = unsafe {mem::zeroed()};
    let p_st: LPSYSTEMTIME = &mut st;

    if unsafe {FileTimeToSystemTime(&ft, p_st )} != 0 {
        let ndt = NaiveDate::from_ymd(st.wYear as i32, st.wMonth as u32, st.wDay as u32)
            .and_hms_milli(st.wHour as u32, st.wMinute as u32, st.wSecond as u32, st.wMilliseconds as u32);
        let dt = OsDateTime::from_utc(ndt, chrono::Utc);
        dt.to_string()
    }
    else {
        "".to_string()
    }
}
 */

