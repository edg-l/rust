use core::str::FromStr;

use super::unsupported;
use crate::ffi::{OsStr, OsString};
use crate::path::{self, PathBuf};
use crate::sys::cvt_io;
use crate::{fmt, io};

pub fn errno() -> i32 {
    edos_rt::sys::errno() as i32
}

pub fn error_string(errno: i32) -> String {
    if errno == 0 {
        "operation successful".to_string()
    } else {
        match errno as u64 {
            1 => "Invalid argument passed to a syscall".to_string(),
            2 => "Memory allocation failed or memory exhausted".to_string(),
            3 => "Bad memory address provided by userspace".to_string(),
            4 => "Invalid or closed file descriptor".to_string(),
            5 => "Operation requires permissions the caller lacks".to_string(),
            6 => "Operation not permitted for the current caller.".to_string(),
            7 => "Requested file or directory does not exist.".to_string(),
            8 => "Attempted to create an entry that already exists.".to_string(),
            9 => "Expected a directory but encountered a non-directory entry.".to_string(),
            10 => "Operation required a regular file but encountered a directory".to_string(),
            11 => "Device or filesystem has no space left for the operation".to_string(),
            12 => "Write attempted on a read-only filesystem or device".to_string(),
            13 => "Generic I/O failure surfaced from the filesystem or storage layer".to_string(),
            _ => "unmapped kernel error code".to_string(),
        }
    }
}

#[inline]
pub fn getcwd() -> io::Result<PathBuf> {
    let result = cvt_io(edos_rt::fs::getcwd())?;
    let path = PathBuf::from(result);
    Ok(path)
}

pub fn chdir(path: &path::Path) -> io::Result<()> {
    cvt_io(edos_rt::fs::chdir(&path.to_string_lossy()))?;
    Ok(())
}

pub struct SplitPaths<'a> {
    iter: core::str::Split<'a, char>,
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = PathBuf;

    fn next(&mut self) -> Option<PathBuf> {
        self.iter.next().map(PathBuf::from)
    }
}

pub fn split_paths(unparsed: &OsStr) -> SplitPaths<'_> {
    let str = unparsed.to_str().expect("non-UTF8 paths not supported");
    SplitPaths { iter: str.split(':') }
}

#[derive(Debug)]
pub struct JoinPathsError;

pub fn join_paths<I, T>(paths: I) -> Result<OsString, JoinPathsError>
where
    I: Iterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut result = Vec::new();
    let mut first = true;

    for path in paths {
        let bytes = path.as_ref().as_encoded_bytes();

        if bytes.contains(&b':') {
            return Err(JoinPathsError);
        }

        if !first {
            result.push(b':');
        }
        result.extend_from_slice(bytes);
        first = false;
    }

    Ok(unsafe { OsString::from_encoded_bytes_unchecked(result) })
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "path segment contains separator `:`".fmt(f)
    }
}

impl crate::error::Error for JoinPathsError {
    fn description(&self) -> &str {
        "failed to join paths"
    }
}

pub fn current_exe() -> io::Result<PathBuf> {
    unsupported()
}

pub fn temp_dir() -> PathBuf {
    PathBuf::from_str("/tmp").unwrap()
}

pub fn home_dir() -> Option<PathBuf> {
    None
}

pub fn exit(code: i32) -> ! {
    edos_rt::process::sys_exit(code)
}

pub fn getpid() -> u32 {
    edos_rt::process::sys_getpid() as u32
}
