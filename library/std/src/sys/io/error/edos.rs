use edos_rt::sys::Errno;

use crate::io;

pub fn errno() -> i32 {
    edos_rt::sys::errno() as u64 as i32
}

pub fn is_interrupted(errno: i32) -> bool {
    Errno::from_raw(errno as u64) == Errno::EINTR
}

pub fn decode_error_kind(code: i32) -> io::ErrorKind {
    crate::sys::error_kind(Errno::from_raw(code as u64))
}

pub fn error_string(errno: i32) -> String {
    format!("OS error {errno}")
}
