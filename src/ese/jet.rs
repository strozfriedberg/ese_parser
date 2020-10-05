//jet.rs
#![allow(deprecated)]
use libc::{uint8_t, uint32_t};
use chrono::naive::{NaiveDateTime, NaiveDate};
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
    filler: uint8_t,
}

impl DateTime {
    pub fn to_string(self) -> String {
        if self.year > 0 {
            let ndt: NaiveDateTime = self.into();
            ndt.to_string()
        } else {
            "".to_string()
        }
    }
}

impl Into<NaiveDateTime> for DateTime {
    fn into(self) -> NaiveDateTime {
        NaiveDate::from_ymd(self.year as i32 + 1900, self.month as u32, self.day as u32)
                    .and_hms(self.hours as u32, self.minutes as u32, self.seconds as u32)
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

