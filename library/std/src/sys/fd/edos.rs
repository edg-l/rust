#![unstable(reason = "not public", issue = "none", feature = "fd")]

use edos_rt::fd::{FstatEntry, IoVec};
use edos_rt::io::sys_read;
use edos_rt::process::dup;

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, Read, SeekFrom};
use crate::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::sys::{cvt, cvt_io, unsupported};

#[allow(unused)]
const fn max_iov() -> usize {
    16
}

/// Describe the caller's buffers to the kernel.
///
/// Capped at [`max_iov`]: a vectored call is allowed to be short, and a caller
/// that has more buffers than the kernel takes at once gets the rest on its
/// next call.
fn iovecs<'a>(bufs: impl Iterator<Item = (*const u8, usize)>) -> Vec<IoVec> {
    // The kernel reads and writes through these pointers, so the provenance has
    // to be exposed rather than stripped with `addr`.
    bufs.take(max_iov())
        .map(|(base, len)| IoVec { base: base.expose_provenance() as u64, len: len as u64 })
        .collect()
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

    pub fn read_buf(&self, mut buf: BorrowedCursor<'_, u8>) -> io::Result<()> {
        // SAFETY: The `read` syscall does not read from the buffer, so it is
        // safe to use `&mut [MaybeUninit<u8>]`.
        let result = cvt(unsafe {
            sys_read(self.inner.raw_fd(), buf.as_mut().as_mut_ptr() as *mut u8, buf.capacity())
        })?;
        // SAFETY: Exactly `result` bytes have been filled.
        unsafe { buf.advance(result as usize) };
        Ok(())
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let iovs = iovecs(bufs.iter().map(|b| (b.as_ptr(), b.len())));
        cvt_io(self.inner.readv(&iovs))
    }

    #[inline]
    pub fn is_read_vectored(&self) -> bool {
        true
    }

    pub fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut me = self;
        (&mut me).read_to_end(buf)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let result = cvt_io(self.inner.write(buf))?;
        Ok(result as usize)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let iovs = iovecs(bufs.iter().map(|b| (b.as_ptr(), b.len())));
        cvt_io(self.inner.writev(&iovs))
    }

    #[inline]
    pub fn is_write_vectored(&self) -> bool {
        true
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let (offset, whence) = match pos {
            SeekFrom::Start(n) => (n as i64, 0u32),
            SeekFrom::Current(n) => (n, 1u32),
            SeekFrom::End(n) => (n, 2u32),
        };
        cvt_io(self.inner.lseek(offset, whence))
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
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
        cvt_io(edos_rt::fd::set_cloexec(self.inner.raw_fd(), true))
    }

    pub fn set_nonblocking(&self, _nonblocking: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn fstat(&self) -> io::Result<FstatEntry> {
        cvt_io(self.inner.fstat())
    }

    pub fn fsync(&self) -> io::Result<()> {
        cvt_io(self.inner.fsync())
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(FileDesc {
            inner: edos_rt::fd::FileDesc::from_raw_fd(
                dup(self.inner.raw_fd()),
                edos_rt::fd::OpenFlags::CREATE,
            ),
        })
    }
}

impl AsRawFd for FileDesc {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.inner.raw_fd() as RawFd
    }
}

impl IntoRawFd for FileDesc {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        // Reading the number out and letting `self` fall out of scope would
        // hand back a descriptor its own drop had just closed.
        self.inner.into_raw_fd() as RawFd
    }
}

impl FromRawFd for FileDesc {
    #[inline]
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        FileDesc {
            inner: edos_rt::fd::FileDesc::from_raw_fd(fd as u64, edos_rt::fd::OpenFlags::NONE),
        }
    }
}

impl From<OwnedFd> for FileDesc {
    fn from(owned: OwnedFd) -> Self {
        unsafe { Self::from_raw_fd(owned.into_raw_fd()) }
    }
}

impl From<FileDesc> for OwnedFd {
    fn from(fd: FileDesc) -> Self {
        unsafe { OwnedFd::from_raw_fd(fd.into_raw_fd()) }
    }
}

impl crate::sys::IntoInner<OwnedFd> for FileDesc {
    fn into_inner(self) -> OwnedFd {
        unsafe { OwnedFd::from_raw_fd(self.into_raw_fd()) }
    }
}

impl crate::sys::FromInner<OwnedFd> for FileDesc {
    fn from_inner(owned_fd: OwnedFd) -> Self {
        unsafe { Self::from_raw_fd(owned_fd.into_raw_fd()) }
    }
}

impl FileDesc {
    pub fn as_fd(&self) -> crate::os::fd::BorrowedFd<'_> {
        unsafe { crate::os::fd::BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}

impl<'a> Read for &'a FileDesc {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (**self).read(buf)
    }
}
