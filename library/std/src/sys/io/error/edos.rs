use edos_rt::sys::Errno;

use crate::io;

pub fn errno() -> i32 {
    edos_rt::sys::errno() as u64 as i32
}

pub fn is_interrupted(_errno: i32) -> bool {
    false
}

pub fn decode_error_kind(code: i32) -> io::ErrorKind {
    let errno: Errno = unsafe { core::mem::transmute(code as u64) };
    match errno {
        Errno::EACCES => io::ErrorKind::PermissionDenied,
        Errno::EEXIST => io::ErrorKind::AlreadyExists,
        Errno::EINVAL => io::ErrorKind::InvalidInput,
        Errno::ENOENT => io::ErrorKind::NotFound,
        Errno::EPERM => io::ErrorKind::PermissionDenied,
        _ => io::ErrorKind::Uncategorized,
    }
}

pub fn error_string(errno: i32) -> String {
    format!("OS error {errno}")
}
