use edos_rt::io::IoResult;
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

/// Maps a kernel error code onto an [`ErrorKind`].
///
/// The kernel's codes are what distinguishes a missing path from a full disk, so
/// this is the single place that translation happens; `sys::io` decodes raw
/// codes through it.
pub fn error_kind(errno: Errno) -> ErrorKind {
    match errno {
        Errno::EACCES | Errno::EPERM => ErrorKind::PermissionDenied,
        Errno::EEXIST => ErrorKind::AlreadyExists,
        Errno::EINVAL => ErrorKind::InvalidInput,
        Errno::ENOENT => ErrorKind::NotFound,
        Errno::ENOTDIR => ErrorKind::NotADirectory,
        Errno::EISDIR => ErrorKind::IsADirectory,
        Errno::ENOSPC => ErrorKind::StorageFull,
        Errno::EROFS => ErrorKind::ReadOnlyFilesystem,
        Errno::EIO => ErrorKind::Other,
        Errno::EINTR => ErrorKind::Interrupted,
        Errno::EAGAIN => ErrorKind::WouldBlock,
        Errno::ENOMEM => ErrorKind::OutOfMemory,
        Errno::EBADF => ErrorKind::InvalidInput,
        Errno::ENOEXEC => ErrorKind::InvalidData,
        Errno::ENOTCONN => ErrorKind::NotConnected,
        Errno::ECONNREFUSED => ErrorKind::ConnectionRefused,
        Errno::EADDRINUSE => ErrorKind::AddrInUse,
        Errno::EPIPE => ErrorKind::BrokenPipe,
        Errno::EAFNOSUPPORT => ErrorKind::Unsupported,
        Errno::EFAULT | Errno::Clear | Errno::UNKNOWN => ErrorKind::Uncategorized,
    }
}

pub fn abort_internal() -> ! {
    edos_rt::process::sys_exit(1)
}

pub fn cvt(t: isize) -> Result<isize, std_io::Error> {
    if t == -1 {
        return Err(error_kind(errno()).into());
    }
    Ok(t)
}

pub fn cvt_io<T>(result: IoResult<T>) -> crate::io::Result<T> {
    result.map_err(|errno| error_kind(errno).into())
}
