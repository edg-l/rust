use edos_rt::io::{IoError, IoResult};
use edos_rt::sys::{Errno, errno};

use crate::io::{self as std_io, ErrorKind};

// SAFETY: must be called only once during runtime initialization.
// NOTE: this is not guaranteed to run, for example when Rust code is called externally.
pub unsafe fn init(_argc: isize, _argv: *const *const u8, _sigpipe: u8) {}

// SAFETY: must be called only once during runtime cleanup.
// NOTE: this is not guaranteed to run, for example when the program aborts.
pub unsafe fn cleanup() {}

pub fn unsupported<T>() -> std_io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> std_io::Error {
    std_io::Error::UNSUPPORTED_PLATFORM
}

pub fn is_interrupted(_code: i32) -> bool {
    false
}

pub fn decode_error_kind(code: i32) -> crate::io::ErrorKind {
    let errno: Errno = unsafe { core::mem::transmute(code as u64) };
    match errno {
        edos_rt::sys::Errno::EACCES => ErrorKind::PermissionDenied,
        edos_rt::sys::Errno::EEXIST => ErrorKind::AlreadyExists,
        edos_rt::sys::Errno::EINVAL => ErrorKind::InvalidInput,
        edos_rt::sys::Errno::ENOENT => ErrorKind::NotFound,
        edos_rt::sys::Errno::EPERM => ErrorKind::PermissionDenied,
        _ => ErrorKind::Uncategorized,
    }
}

pub fn abort_internal() -> ! {
    edos_rt::process::sys_exit(1)
}

pub fn cvt(t: isize) -> Result<isize, std_io::Error> {
    if t == -1 {
        let err = errno();
        return Err(decode_error_kind(err as u64 as i32).into());
    }
    Ok(t)
}

pub fn cvt_io<T>(result: IoResult<T>) -> crate::io::Result<T> {
    match result {
        Ok(x) => Ok(x),
        Err(error) => Err(match error {
            IoError::InvalidInput => ErrorKind::InvalidInput,
            IoError::OutOfMemory => ErrorKind::OutOfMemory,
            IoError::Interrupted => ErrorKind::Interrupted,
            _ => ErrorKind::Uncategorized,
        }
        .into()),
    }
}
