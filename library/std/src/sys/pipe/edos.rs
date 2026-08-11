use crate::io;
use crate::sys::fd::FileDesc;

pub type Pipe = FileDesc;

pub fn pipe() -> io::Result<(Pipe, Pipe)> {
    let (read_fd, write_fd) = edos_rt::process::pipe().unwrap();
    Ok((
        Pipe { inner: edos_rt::fd::FileDesc::from_raw_fd(read_fd, edos_rt::fd::OpenFlags::CREATE) },
        Pipe {
            inner: edos_rt::fd::FileDesc::from_raw_fd(write_fd, edos_rt::fd::OpenFlags::CREATE),
        },
    ))
}
