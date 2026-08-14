use edos_rt::io::IoResult;
use edos_rt::sys::Errno;

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
        Errno::ESPIPE => ErrorKind::NotSeekable,
        Errno::EBUSY => ErrorKind::ResourceBusy,
        Errno::ELOOP => ErrorKind::FilesystemLoop,
        Errno::EINPROGRESS | Errno::EALREADY => ErrorKind::InProgress,
        Errno::ENOSYS | Errno::EOPNOTSUPP => ErrorKind::Unsupported,
        Errno::ETIMEDOUT => ErrorKind::TimedOut,
        Errno::ECONNRESET => ErrorKind::ConnectionReset,
        Errno::ECONNABORTED => ErrorKind::ConnectionAborted,
        Errno::EHOSTUNREACH => ErrorKind::HostUnreachable,
        Errno::ENETUNREACH => ErrorKind::NetworkUnreachable,
        Errno::EADDRNOTAVAIL => ErrorKind::AddrNotAvailable,
        Errno::ENOTEMPTY => ErrorKind::DirectoryNotEmpty,
        Errno::EXDEV => ErrorKind::CrossesDevices,
        Errno::EMLINK => ErrorKind::TooManyLinks,
        Errno::ENAMETOOLONG => ErrorKind::InvalidFilename,
        Errno::EFBIG | Errno::E2BIG => ErrorKind::FileTooLarge,
        Errno::ESRCH | Errno::ECHILD => ErrorKind::NotFound,
        Errno::EDOM | Errno::ERANGE | Errno::EOVERFLOW => ErrorKind::InvalidInput,
        Errno::ENOTSOCK | Errno::ENOTTY => ErrorKind::InvalidInput,
        Errno::EMSGSIZE => ErrorKind::InvalidInput,
        Errno::ENFILE | Errno::EMFILE | Errno::ENOBUFS => ErrorKind::QuotaExceeded,
        // ENXIO is a named pipe opened for writing with no reader, and EISCONN
        // a `connect` on a socket that already has one. No `ErrorKind` names
        // either, and the unix mapping leaves them uncategorised too, so a
        // caller that needs to tell them apart reads the raw code. ENODEV joins
        // them: it names a device that is absent rather than a path that is.
        Errno::ENXIO
        | Errno::EISCONN
        | Errno::EFAULT
        | Errno::ENODEV
        | Errno::Clear
        | Errno::UNKNOWN => ErrorKind::Uncategorized,
    }
}

pub fn abort_internal() -> ! {
    edos_rt::process::sys_exit(1)
}

/// Splits a raw syscall return into a result and an [`std_io::Error`].
///
/// A failure is any return in the `[-4095, -1]` window, not only `-1`: the
/// kernel puts the code itself in the return register, so testing the legacy
/// sentinel alone lets every other code through as a valid result — a count, a
/// length, or an address the caller then uses. Reading the code from the return
/// also avoids the `SYS_ERRNO` round trip, which a signal handler could race.
pub fn cvt(t: isize) -> Result<isize, std_io::Error> {
    match edos_rt::sys::sys_result(t as u64) {
        Ok(_) => Ok(t),
        Err(e) => Err(error_kind(e).into()),
    }
}

pub fn cvt_io<T>(result: IoResult<T>) -> crate::io::Result<T> {
    result.map_err(|errno| error_kind(errno).into())
}
