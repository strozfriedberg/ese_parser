//jet.rs
#![allow(deprecated)]
use libc::{uint8_t, uint32_t};
use std::mem;
use chrono;
use chrono::TimeZone;
use chrono::naive::NaiveDate;
use winapi::um::timezoneapi::GetTimeZoneInformation;
use strum::Display;

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

type OsDateTime = chrono::DateTime<chrono::Utc>;

impl DateTime {
    pub fn to_string(self) -> String {
        if self.year > 0 {
            let dt: OsDateTime = self.into();
            dt.to_string()
        } else {
            "".to_string()
        }
    }
}

impl Into<OsDateTime> for DateTime {
    fn into(self) -> OsDateTime {
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

        OsDateTime::from(chrono::FixedOffset::east(offset).from_local_datetime(&ndt).unwrap())
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

