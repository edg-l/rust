use crate::io;
use crate::sys::fd::FileDesc;

pub type Pipe = FileDesc;

pub fn pipe() -> io::Result<(Pipe, Pipe)> {
    let (fd0, fd1) = edos_rt::process::pipe().unwrap();
    Ok((
        Pipe {
            inner: edos_rt::fd::FileDesc::from_raw_fd(fd0, edos_rt::fd::OpenFlags::Create),
        },
        Pipe {
            inner: edos_rt::fd::FileDesc::from_raw_fd(fd1, edos_rt::fd::OpenFlags::Create),
        },
    ))
}
