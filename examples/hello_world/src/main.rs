#![feature(extern_types)]
#![feature(start)]

#![deny(warnings)]

#![windows_subsystem="console"]
#![no_std]
#![cfg_attr(any(target_os="dos", windows), no_main)]

extern crate pc_atomics;
extern crate rlibc_ext;

mod no_std {
    #[panic_handler]
    fn panic_handler(info: &core::panic::PanicInfo) -> ! { panic_no_std::panic(info, b'P') }
}

#[cfg(any(target_os="dos", windows))]
extern {
    type PEB;
}

#[cfg(all(not(target_os="dos"), not(windows)))]
#[start]
fn main(_: isize, _: *const *const u8) -> isize {
    start();
    0
}

#[cfg(any(target_os="dos", windows))]
#[allow(non_snake_case)]
#[no_mangle]
extern "stdcall" fn mainCRTStartup(_: *const PEB) -> u64 {
    start();
    0
}

use print_no_std::println;

fn start() {
    #[cfg(target_os="dos")]
    dos_cp::CodePage::load_or_exit_with_msg(99);
    println!("Hello, World!");
}
