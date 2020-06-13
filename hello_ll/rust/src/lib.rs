#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![no_std]

#[macro_use]
extern crate alloc;

use bento;
use bento::fuse::*;
use bento::c_str;
use bento::println;

pub mod hello_ll;

use hello_ll::HelloFS;

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    HelloFS.register();
}

#[no_mangle]
pub fn rust_exit() {
    HelloFS.unregister();
}
