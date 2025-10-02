#![unstable(reason = "not public", issue = "none", feature = "fd")]

use edos_rt::fd::FstatEntry;
use edos_rt::io::sys_read;

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, Read, SeekFrom};
use crate::sys::{cvt, cvt_io, unsupported};

#[allow(unused)]
const fn max_iov() -> usize {
    16
}

#[derive(Debug)]
pub struct FileDesc {
    pub(crate) inner: edos_rt::fd::FileDesc,
}

impl FileDesc {
    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let result = cvt_io(self.inner.read(buf))?;
        Ok(result as usize)
    }

    pub fn read_buf(&self, mut buf: BorrowedCursor<'_>) -> io::Result<()> {
        // SAFETY: The `read` syscall does not read from the buffer, so it is
        // safe to use `&mut [MaybeUninit<u8>]`.
        let result = cvt(unsafe {
            sys_read(self.inner.raw_fd(), buf.as_mut().as_mut_ptr() as *mut u8, buf.capacity())
        })?;
        // SAFETY: Exactly `result` bytes have been filled.
        unsafe { buf.advance_unchecked(result as usize) };
        Ok(())
    }

    pub fn read_vectored(&self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        unsupported()
    }

    #[inline]
    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut me = self;
        (&mut me).read_to_end(buf)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let result = cvt_io(self.inner.write(buf))?;
        Ok(result as usize)
    }

    pub fn write_vectored(&self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        unsupported()
    }

    #[inline]
    pub fn is_write_vectored(&self) -> bool {
        true
    }

    pub fn seek(&self, _pos: SeekFrom) -> io::Result<u64> {
        unsupported()
    }

    pub fn tell(&self) -> io::Result<u64> {
        unsupported()
    }

    pub fn duplicate(&self) -> io::Result<FileDesc> {
        self.duplicate_path(&[])
    }

    pub fn duplicate_path(&self, _path: &[u8]) -> io::Result<FileDesc> {
        unsupported()
    }

    pub fn nonblocking(&self) -> io::Result<bool> {
        Ok(false)
    }

    pub fn set_cloexec(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn set_nonblocking(&self, _nonblocking: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn fstat(&self) -> io::Result<FstatEntry> {
        cvt_io(self.inner.fstat())
    }
}

impl<'a> Read for &'a FileDesc {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (**self).read(buf)
    }
}
