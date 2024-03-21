#![deny(warnings)]
#![allow(clippy::unnecessary_literal_unwrap)]

#![no_std]

use core::fmt::{self};

#[doc(hidden)]
pub use core::write as std_write;
#[doc(hidden)]
pub use core::writeln as std_writeln;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum WriteErrorStrategy {
    Passthrough,
    Panic,
    //Ignore
}

#[cfg(all(not(target_os="dos"), windows))]
mod winapi {
    use core::cmp::min;
    use core::fmt::{self};
    use core::mem::size_of;
    use core::ptr::null_mut;
    use crate::WriteErrorStrategy;
    use errno_no_std::errno;
    use iter_identify_first_last::IteratorIdentifyFirstLastExt;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::consoleapi::{GetConsoleMode, WriteConsoleW};
    use winapi::um::fileapi::WriteFile;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    pub type StdHandle = u32;

    pub const STDOUT: StdHandle = STD_OUTPUT_HANDLE;

    pub const STDERR: StdHandle = STD_ERROR_HANDLE;

    pub fn write_str(std_handle: StdHandle, s: &str, error_strategy: WriteErrorStrategy) -> fmt::Result {
        let handle = unsafe { GetStdHandle(std_handle) };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            match error_strategy {
                WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                WriteErrorStrategy::Panic => panic!("cannot get std handle"),
                //WriteErrorStrategy::Ignore => return Ok(()),
            }
        }
        let mut mode: u32 = 0;
        if unsafe { GetConsoleMode(handle, &mut mode as *mut _) } == 0 {
            let mut s = s;
            while !s.is_empty() {
                let mut written: u32 = 0;
                assert!(size_of::<u32>() <= size_of::<usize>());
                let len = min(s.len(), u32::MAX as usize);
                if unsafe { WriteFile(
                    handle,
                    s.as_ptr() as _,
                    len as u32,
                    &mut written as *mut _,
                    null_mut()
                ) } == 0 {
                    match error_strategy {
                        WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                        WriteErrorStrategy::Panic => Err(errno()).unwrap(),
                        //WriteErrorStrategy::Ignore => return Ok(()),
                    }
                }
                assert_eq!(written, len as u32);
                s = &s[len ..];
            }
        } else {
            let mut buf = [0u16; 128];
            let mut buf_offset = 0;
            for (is_last, c) in s.chars().identify_last() {
                buf_offset += c.encode_utf16(&mut buf[buf_offset ..]).len();
                if is_last || buf.len() - buf_offset < 2 {
                    let mut written: DWORD = 0;
                    if unsafe { WriteConsoleW(
                        handle,
                        buf.as_ptr() as _,
                        buf_offset.try_into().unwrap(),
                        &mut written as *mut _,
                        null_mut()
                    ) } == 0 {
                        match error_strategy {
                            WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                            WriteErrorStrategy::Panic => Err(errno()).unwrap(),
                            //WriteErrorStrategy::Ignore => return Ok(()),
                        }
                    }
                    assert_eq!(written, buf_offset.try_into().unwrap());
                    buf_offset = 0;
                }
            }
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod posix {
    use crate::WriteErrorStrategy;
    use core::fmt::{self};
    use errno_no_std::{Errno, errno};
    use libc::{c_int, iconv, iconv_t, iconv_open, nl_langinfo, iconv_close, CODESET, E2BIG};

    pub type StdHandle = c_int;

    pub const STDOUT: StdHandle = 1;

    pub const STDERR: StdHandle = 2;

    const ICONV_ERR: iconv_t = (-1isize) as usize as iconv_t;

    pub fn write_str(std_handle: StdHandle, s: &str, error_strategy: WriteErrorStrategy) -> fmt::Result {
        let conv = unsafe { iconv_open(nl_langinfo(CODESET), b"UTF-8\0".as_ptr() as _) };
        if conv == ICONV_ERR {
            match error_strategy {
                WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                WriteErrorStrategy::Panic => Err(errno()).unwrap(),
                //WriteErrorStrategy::Ignore => return Ok(()),
            }
        }
        let mut buf = [0u8; 128];
        let mut iconv_in = s.as_ptr();
        let mut iconv_in_len = s.len();
        loop {
            let mut iconv_out = (buf[..]).as_mut_ptr();
            let mut iconv_out_len = buf.len();
            let iconv_res = unsafe { iconv(
                conv,
                (&mut iconv_in) as *mut _ as _,
                (&mut iconv_in_len) as *mut _,
                (&mut iconv_out) as *mut _ as _,
                (&mut iconv_out_len) as *mut _,
            ) };
            let stop = if iconv_res == (-1isize) as usize {
                let err = errno();
                if err != Errno(E2BIG) {
                    match error_strategy {
                        WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                        WriteErrorStrategy::Panic => Err(err).unwrap(),
                        //WriteErrorStrategy::Ignore => return Ok(()),
                    }
                }
                false
            } else {
                debug_assert_eq!(iconv_in_len, 0);
                true
            };
            let written = buf.len() - iconv_out_len;
            if unsafe { libc::write(std_handle, buf.as_ptr() as _, written) } == -1 {
                let err = errno();
                match error_strategy {
                    WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                    WriteErrorStrategy::Panic => Err(err).unwrap(),
                    //WriteErrorStrategy::Ignore => return Ok(()),
                }
            }
            if stop { break; }
        }
        if unsafe { iconv_close(conv) } == -1 {
            let err = errno();
            match error_strategy {
                WriteErrorStrategy::Passthrough => return Err(fmt::Error),
                WriteErrorStrategy::Panic => Err(err).unwrap(),
                //WriteErrorStrategy::Ignore => return Ok(()),
            }
        }
        Ok(())
    }
}

#[cfg(target_os="dos")]
mod dos {
    use crate::WriteErrorStrategy;
    use core::fmt::{self};
    use core::fmt::Write as fmt_Write;
    use dos_cp::DosStdout;

    pub type StdHandle = ();

    pub const STDOUT: () = ();

    pub const STDERR: () = ();

    pub fn write_str((): StdHandle, s: &str, error_strategy: WriteErrorStrategy) -> fmt::Result {
        if let Err(e) = (DosStdout { panic: error_strategy == WriteErrorStrategy::Panic }).write_str(s) {
            if error_strategy == WriteErrorStrategy::Passthrough { Err(e) } else { Ok(()) }
        } else {
            Ok(())
        }
    }
}

#[cfg(all(not(target_os="dos"), windows))]
use winapi::*;
#[cfg(not(windows))]
use posix::*;
#[cfg(target_os="dos")]
use dos::*;

pub struct Stdout { pub panic: bool }

pub struct Stderr { pub panic: bool }

impl Stdout {
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        <Self as fmt::Write>::write_fmt(self, args)
    }
}

impl Stderr {
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        <Self as fmt::Write>::write_fmt(self, args)
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let error_strategy = if self.panic {
            WriteErrorStrategy::Panic
        } else {
            WriteErrorStrategy::Passthrough
        };
        write_str(STDOUT, s, error_strategy)
    }
}

impl fmt::Write for Stderr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let error_strategy = if self.panic {
            WriteErrorStrategy::Panic
        } else {
            WriteErrorStrategy::Passthrough
        };
        write_str(STDERR, s, error_strategy)
    }
}

#[macro_export]
macro_rules! print {
    (
        $($arg:tt)*
    ) => {
        $crate::std_write!($crate::Stdout { panic: true }, $($arg)*).unwrap()
    };
}

#[macro_export]
macro_rules! println {
    (
    ) => {
        $crate::std_writeln!($crate::Stdout { panic: true }).unwrap()
    };
    (
        $($arg:tt)*
    ) => {
        $crate::std_writeln!($crate::Stdout { panic: true }, $($arg)*).unwrap()
    };
}

#[macro_export]
macro_rules! eprint {
    (
        $($arg:tt)*
    ) => {
        $crate::std_write!($crate::Stderr { panic: true }, $($arg)*).unwrap()
    };
}

#[macro_export]
macro_rules! eprintln {
    (
    ) => {
        $crate::std_writeln!($crate::Stderr { panic: true }).unwrap()
    };
    (
        $($arg:tt)*
    ) => {
        $crate::std_writeln!($crate::Stderr { panic: true }, $($arg)*).unwrap()
    };
}
