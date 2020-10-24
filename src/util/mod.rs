//util\mod.rs
pub mod config;
pub mod dumper;

use std::mem;
use std::slice;

/// # Safety
///
/// use slice::from_raw_parts_mut
pub unsafe fn any_as_u8_slice<'a, T: Sized>(p: &'a &mut T) -> &'a mut [u8] {
    slice::from_raw_parts_mut((*p as *const T) as *mut u8, mem::size_of::<T>())
}

/// # Safety
///
/// use slice::from_raw_parts_mut
pub unsafe fn any_as_u32_slice<'a, T: Sized>(p: &'a &mut T) -> &'a mut [u32] {
    slice::from_raw_parts_mut((*p as *const T) as *mut u32, mem::size_of::<T>() / mem::size_of::<u32>())
}

#[macro_export]
macro_rules! expect_eq {
    ($left:expr, $right:expr) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    error!(r#"expectation failed: `({} == {})`
  left: `{:?}`,
 right: `{:?}`"#, stringify!($left), stringify!($right), &*left_val, &*right_val)
                }
            }
        }
    });
    ($left:expr, $right:expr,) => ({
        expect_eq!($left, $right)
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    error!(r#"expectation failed: `({} == {})`
  left: `{:?}`,
 right: `{:?}`: {}"#, stringify!($left), stringify!($right), &*left_val, &*right_val,
                           format_args!($($arg)+))
                }
            }
        }
    });
}

pub mod reader;
