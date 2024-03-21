#![feature(start)]

#![deny(warnings)]

#![no_std]

use core::panic::PanicInfo;
use panic_no_std::panic;

#[cfg(windows)]
#[link(name="msvcrt")]
extern { }

#[panic_handler]
fn panic_hanlder(info: &PanicInfo) -> ! {
    panic(info, 0)
}

#[no_mangle]
extern fn rust_eh_personality() { }

use print_no_std::println;

#[start]
pub fn main(_argc: isize, _argv: *const *const u8) -> isize {
    println!("message");
    0
}
