use edos_rt::io::{sys_read, sys_write};

use crate::io::{self};
use crate::sys::os::errno;
use crate::sys::{cvt, decode_error_kind};

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = unsafe { sys_read(0, buf.as_mut_ptr(), buf.len()) };

        if count != -1 { Ok(count as usize) } else { Err(decode_error_kind(errno()).into()) }
    }

    #[inline]
    fn read_buf(&mut self, mut buf: core::io::BorrowedCursor<'_>) -> io::Result<()> {
        // SAFETY: The `read` syscall does not read from the buffer, so it is
        // safe to use `&mut [MaybeUninit<u8>]`.
        let result =
            cvt(unsafe { sys_read(0, buf.as_mut().as_mut_ptr() as *mut u8, buf.capacity()) })?;
        // SAFETY: Exactly `result` bytes have been filled.
        unsafe { buf.advance_unchecked(result as usize) };
        Ok(())
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = unsafe { sys_write(1, buf.as_ptr(), buf.len()) };

        if count != -1 { Ok(count as usize) } else { Err(decode_error_kind(errno()).into()) }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub const fn new() -> Stderr {
        Stderr
    }
}

impl io::Write for Stderr {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = unsafe { sys_write(2, buf.as_ptr(), buf.len()) };

        if count != -1 { Ok(count as usize) } else { Err(decode_error_kind(errno()).into()) }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub const STDIN_BUF_SIZE: usize = crate::sys::io::DEFAULT_BUF_SIZE;

pub fn is_ebadf(_err: &io::Error) -> bool {
    true
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(Stderr)
}
