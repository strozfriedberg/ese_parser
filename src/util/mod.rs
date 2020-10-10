pub mod config;
pub mod dumper;
pub mod reader;

use std::mem;
use std::slice;

pub unsafe fn any_as_u8_slice<'a, T: Sized>(p: &'a &mut T) -> &'a mut [u8] {
    slice::from_raw_parts_mut((*p as *const T) as *mut u8, mem::size_of::<T>())
}
