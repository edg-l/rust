use edos_rt::fd::{PollFd, PollState, poll};
use edos_rt::io::cvt;

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::sys::fd::FileDesc;
use crate::sys::{cvt_io, unsupported};
use crate::sys_common::{FromInner, IntoInner};

#[derive(Debug)]
pub struct AnonPipe(FileDesc);

impl AnonPipe {
    pub fn try_clone(&self) -> io::Result<Self> {
        unsupported() // TODO: implement this
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    pub fn read_buf(&self, buf: BorrowedCursor<'_>) -> io::Result<()> {
        self.0.read_buf(buf)
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }

    pub fn is_read_vectored(&self) -> bool {
        self.0.is_read_vectored()
    }

    pub fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.0.read_to_end(buf)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        self.0.is_write_vectored()
    }

    pub fn into_raw_fd(self) -> FileDesc {
        self.0
    }
}

pub fn read2(p1: AnonPipe, v1: &mut Vec<u8>, p2: AnonPipe, v2: &mut Vec<u8>) -> io::Result<()> {
    let mut entries = [
        PollFd {
            fd: p1.0.inner.raw_fd(),
            interests: PollState { readable: true, writable: false, error: true },
            result: PollState::default(),
        },
        PollFd {
            fd: p1.0.inner.raw_fd(),
            interests: PollState { readable: true, writable: false, error: true },
            result: PollState::default(),
        },
    ];
    let _result = cvt_io(cvt(poll(&mut entries, 0) as isize))?;

    if entries[0].result.readable {
        p1.read(v1)?;
    }

    if entries[1].result.readable {
        p2.read(v2)?;
    }

    Ok(())
}

impl FromInner<FileDesc> for AnonPipe {
    fn from_inner(inner: FileDesc) -> Self {
        Self(inner)
    }
}

impl IntoInner<FileDesc> for AnonPipe {
    fn into_inner(self) -> FileDesc {
        self.0
    }
}
