use core::mem::size_of;

use crate::bindings::*;

pub const FUSE_NAME_OFFSET: usize = 24;

pub const FUSE_MAX_MAX_PAGES: u32 = 256;
pub const FUSE_DEFAULT_MAX_PAGES_PER_REQ: u32 = 32;

pub const FUSE_BUFFER_HEADER_SIZE: u32 = 0x1000;

/// Calculate the next correct `fuse_dirent` alignment after the provided offset.
pub fn fuse_dirent_align(x: usize) -> usize {
    let size = size_of::<u64>();
    let left = x + size - 1;
    let right = !(size - 1);
    let ret = left & right;
    return ret;
}
