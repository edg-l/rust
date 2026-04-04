use crate::path::{self, PathBuf};
use crate::sys::{cvt_io};
use crate::io;

pub fn getcwd() -> io::Result<PathBuf> {
    let result = cvt_io(edos_rt::fs::getcwd())?;
    Ok(PathBuf::from(result))
}

pub fn chdir(path: &path::Path) -> io::Result<()> {
    cvt_io(edos_rt::fs::chdir(&path.to_string_lossy()))?;
    Ok(())
}
