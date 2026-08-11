#![stable(feature = "rust1", since = "1.0.0")]

use crate::fs::File;
use crate::io;
#[stable(feature = "rust1", since = "1.0.0")]
pub use crate::os::fd::*;
use crate::sys::{AsInner, cvt_io};

#[stable(feature = "rust1", since = "1.0.0")]
pub trait FileExt {
    #[stable(feature = "rust1", since = "1.0.0")]
    fn ioctl(&self, request: u64, arg: u64, arg_len: usize, flags: u64) -> io::Result<u64>;
}

#[stable(feature = "rust1", since = "1.0.0")]
impl FileExt for File {
    fn ioctl(&self, request: u64, arg: u64, arg_len: usize, flags: u64) -> io::Result<u64> {
        cvt_io(self.as_inner().0.inner.ioctl(request, arg, arg_len, flags))
    }
}
