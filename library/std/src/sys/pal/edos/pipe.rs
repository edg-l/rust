use edos_rt::fd::{PollFd, PollState, poll};
use edos_rt::io::cvt;

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::sys::cvt_io;
use crate::sys::fd::FileDesc;
use crate::sys::{FromInner, IntoInner};

#[derive(Debug)]
pub struct AnonPipe(FileDesc);

pub fn anon_pipe() -> io::Result<(AnonPipe, AnonPipe)> {
    // The only known way right now to create atomically set the CLOEXEC flag is
    // to use the `pipe2` syscall. This was added to Linux in 2.6.27, glibc 2.9
    // and musl 0.9.3, and some other targets also have it.
    let (fd0, fd1) = edos_rt::process::pipe().unwrap();
    Ok((
        AnonPipe(FileDesc {
            inner: edos_rt::fd::FileDesc::from_raw_fd(fd0, edos_rt::fd::OpenFlags::Create),
        }),
        AnonPipe(FileDesc {
            inner: edos_rt::fd::FileDesc::from_raw_fd(fd1, edos_rt::fd::OpenFlags::Create),
        }),
    ))
}

impl AnonPipe {
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
            interests: PollState {
                readable: true,
                writable: false,
                error: true,
                hangup: false,
                invalid: false,
            },
            result: PollState::default(),
        },
        PollFd {
            fd: p1.0.inner.raw_fd(),
            interests: PollState {
                readable: true,
                writable: false,
                error: true,
                hangup: false,
                invalid: false,
            },
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
