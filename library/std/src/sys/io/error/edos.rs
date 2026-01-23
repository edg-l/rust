use edos_rt::sys::Errno;

use crate::io::ErrorKind;
use crate::sys::io::RawOsError;

pub fn errno() -> RawOsError {
    edos_rt::sys::errno() as RawOsError
}

pub fn is_interrupted(_code: RawOsError) -> bool {
    false
}

pub fn decode_error_kind(code: RawOsError) -> ErrorKind {
    let errno: Errno = unsafe { core::mem::transmute(code as u64) };
    match errno {
        Errno::EACCES => ErrorKind::PermissionDenied,
        Errno::EEXIST => ErrorKind::AlreadyExists,
        Errno::EINVAL => ErrorKind::InvalidInput,
        Errno::ENOENT => ErrorKind::NotFound,
        Errno::EPERM => ErrorKind::PermissionDenied,
        _ => ErrorKind::Uncategorized,
    }
}

pub fn error_string(errno: RawOsError) -> String {
    format!("os error {}", errno)
}
