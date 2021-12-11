#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use chrono::{DateTime, Utc};

// implementation is taken from ReacOS: dll/win32/oleaut32/variant.c

#[repr(C)]
#[derive(PartialEq, Debug, Copy, Clone, Default)]
pub struct SYSTEMTIME {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}

fn IsLeapYear(y: u16) -> bool {
    ((y % 4) == 0) && (((y % 100) != 0) || ((y % 400) == 0))
}

const DATE_MAX : i32 = 2958465;
const DATE_MIN : i32 = -657434;

// Convert a VT_DATE value to a Julian Date
fn VARIANT_JulianFromDate(dateIn: i32) -> i32 {
	let mut julian_days = dateIn;

	julian_days -= DATE_MIN; // Convert to + days from 1 Jan 100 AD
	julian_days += 1757585;  // Convert to + days from 23 Nov 4713 BC (Julian)
	julian_days
}

// Convert a Julian date to Day/Month/Year - from PostgreSQL
fn VARIANT_DMYFromJulian(jd: i32, year: &mut u16, month: &mut u16, day: &mut u16) {
	let mut l = jd + 68569;
	let n = l * 4 / 146097;
	l -= (n * 146097 + 3) / 4;
	let i = (4000 * (l + 1)) / 1461001;
	l += 31 - (i * 1461) / 4;
	let j = (l * 80) / 2447;
	*day = (l - (j * 2447) / 80) as u16;
	l = j / 11;
	*month = ((j + 2) - (12 * l)) as u16;
	*year = (100 * (n - 49) + i + l) as u16;
}

fn VARIANT_RollUdate(st: &mut SYSTEMTIME)
{
	let days = vec![ 0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 ];

	// interpret values signed
	let mut iYear : i16 = st.wYear as i16;
    let mut iMonth : i16 = st.wMonth as i16;
    let mut iDay : i16 = st.wDay as i16;
    let mut iHour : i16 = st.wHour as i16;
    let mut iMinute : i16 = st.wMinute as i16;
    let mut iSecond : i16 = st.wSecond as i16;

	if iYear > 9999 || iYear < -9999 {
		return ; // Invalid value
    }
	// Year 0 to 29 are treated as 2000 + year
	if (0..30).contains(&iYear) {
		iYear += 2000;
    }
	// Remaining years < 100 are treated as 1900 + year
	else if (30..100).contains(&iYear) {
		iYear += 1900;
    }

	iMinute += iSecond / 60;
	iSecond %= 60;
	iHour += iMinute / 60;
	iMinute %= 60;
	iDay += iHour / 24;
	iHour %= 24;
	iYear += iMonth / 12;
	iMonth %= 12;
	if iMonth <= 0 { iMonth += 12; iYear-=1; }
	while iDay > days[iMonth as usize] {
		if iMonth == 2 && IsLeapYear(iYear as u16) {
			iDay -= 29;
        } else {
			iDay -= days[iMonth as usize];
        }
		iMonth+=1;
		iYear += iMonth / 12;
		iMonth %= 12;
	}
	while iDay <= 0 {
		iMonth-=1;
		if iMonth <= 0 { iMonth += 12; iYear-=1; }
		if iMonth == 2 && IsLeapYear(iYear as u16) {
			iDay += 29;
        } else {
			iDay += days[iMonth as usize];
        }
	}

	if iSecond < 0 { iSecond += 60; iMinute-=1; }
	if iMinute < 0 { iMinute += 60; iHour-=1; }
	if iHour < 0 { iHour += 24; iDay-=1; }
	if iYear <= 0 { iYear += 2000; }

	st.wYear = iYear as u16;
	st.wMonth = iMonth as u16;
	st.wDay = iDay as u16;
	st.wHour = iHour as u16;
	st.wMinute = iMinute as u16;
	st.wSecond = iSecond as u16;
}

/// Converts a u64 filetime to a DateTime<Utc>
pub fn get_date_time_from_filetime(filetime: u64) -> DateTime<Utc> {
    const UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH: i128 = 11644473600;
    const UNIX_EPOCH_NANOS: i128 = UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH * 1_000_000_000;
    let filetime_nanos: i128 = filetime as i128 * 100;

    // Add nanoseconds to timestamp via Duration
    DateTime::<Utc>::from_utc(
        chrono::NaiveDate::from_ymd(1970, 1, 1).and_hms_nano(0, 0, 0, 0)
            + chrono::Duration::nanoseconds((filetime_nanos - UNIX_EPOCH_NANOS) as i64),
        Utc,
    )
}

pub fn VariantTimeToSystemTime(dateIn: f64, st: &mut SYSTEMTIME) -> bool {
	if dateIn <= (DATE_MIN as f64 - 1.0) || dateIn >= (DATE_MAX as f64 + 1.0) {
		return false;
    }

	let mut datePart;
    if dateIn < 0.0 {
        datePart = dateIn.ceil();
     } else {
         datePart = dateIn.floor();
     }
	// Compensate for int truncation (always downwards)
	let mut timePart : f64 = (dateIn - datePart).abs() + 0.00000000001;
	if timePart >= 1.0 {
		timePart -= 0.00000000001;
    }

	// Date
	let julianDays = VARIANT_JulianFromDate(dateIn as i32);
	VARIANT_DMYFromJulian(julianDays, &mut st.wYear, &mut st.wMonth, &mut st.wDay);

	datePart = (datePart + 1.5) / 7.0;
	st.wDayOfWeek = ((datePart - datePart.floor()) * 7.0) as u16;
	if st.wDayOfWeek == 0 {
		st.wDayOfWeek = 5;
    } else if st.wDayOfWeek == 1 {
		st.wDayOfWeek = 6;
    } else {
		st.wDayOfWeek -= 2;
    }

	// Time
	timePart *= 24.0;
	st.wHour = timePart as u16;
	timePart -= st.wHour as f64;
	timePart *= 60.0;
	st.wMinute = timePart as u16;
	timePart -= st.wMinute as f64;
	timePart *= 60.0;
	st.wSecond = timePart as u16;
	timePart -= st.wSecond as f64;
	st.wMilliseconds = 0;
	if timePart > 0.5 {
		// Round the milliseconds, adjusting the time/date forward if needed
		if st.wSecond < 59 {
			st.wSecond+=1;
        } else {
			st.wSecond = 0;
			if st.wMinute < 59 {
				st.wMinute+=1;
            } else {
				st.wMinute = 0;
				if st.wHour < 23 {
					st.wHour+=1;
                } else {
					st.wHour = 0;
					// Roll over a whole day
                    st.wDay += 1;
					if st.wDay > 28 {
						VARIANT_RollUdate(st);
                    }
				}
			}
		}
	}
	true
}

#[test]
fn test_vartimes() {
	let t1 : f64 = 44_286.466_608_796_3; 
	let mut st : SYSTEMTIME = unsafe { std::mem::MaybeUninit::<SYSTEMTIME>::zeroed().assume_init() };
	VariantTimeToSystemTime(t1, &mut st);
	assert_eq!(st.wYear, 2021);
	assert_eq!(st.wMonth, 3);
	assert_eq!(st.wDayOfWeek, 3);
	assert_eq!(st.wDay, 31);
	assert_eq!(st.wHour, 11);
	assert_eq!(st.wMonth, 3);
	assert_eq!(st.wSecond, 55);
	assert_eq!(st.wMilliseconds, 0);
}

#[cfg(target_os = "windows")]
#[test]
fn test_curr_time_with_API() {
	use std::mem::MaybeUninit;
	extern "C" {
		pub fn VariantTimeToSystemTime(vtime: f64, lpSystemTime: *mut SYSTEMTIME) -> ::std::os::raw::c_int;
		pub fn SystemTimeToVariantTime(lpSystemTime: *mut SYSTEMTIME, vtime: *mut f64) -> ::std::os::raw::c_int;
		pub fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
	}

    let mut st_orig : SYSTEMTIME;
    let mut st_our : SYSTEMTIME;
    let st_sys : SYSTEMTIME;
    unsafe {
        let mut st = MaybeUninit::<SYSTEMTIME>::zeroed();

        // get local time
        GetLocalTime(st.as_mut_ptr());
        st_orig = st.assume_init();
        st_orig.wMilliseconds = 0;

        // get orig vtime
        let mut vtime : f64 = 0.0;
        SystemTimeToVariantTime(st.as_mut_ptr(), &mut vtime);

        // vtime to SYSTEMTIME using API
        st = MaybeUninit::<SYSTEMTIME>::zeroed();
        VariantTimeToSystemTime(vtime, st.as_mut_ptr());
        st_sys = st.assume_init();

        // vtime to SYSTEMTIME using own impl
        st_our = MaybeUninit::<SYSTEMTIME>::zeroed().assume_init();
        VariantTimeToSystemTime(vtime, &mut st_our);
    }
    assert_eq!(st_orig, st_sys);
    assert_eq!(st_orig, st_our);
}
