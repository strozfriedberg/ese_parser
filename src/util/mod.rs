//util\mod.rs
pub mod config;
pub mod dumper;

use std::mem;
use std::slice;

/// # Safety
///
/// use slice::from_raw_parts_mut
#[allow(clippy::mut_from_ref)]
pub unsafe fn _any_as_slice<'a, U: Sized, T: Sized>(p: &'a &mut T) -> &'a mut [U] {
    slice::from_raw_parts_mut(
        (*p as *const T) as *mut U,
        mem::size_of::<T>() / mem::size_of::<U>(),
    )
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
