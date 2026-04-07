use crate::os::fd::AsRawFd;

pub fn is_terminal(fd: &impl AsRawFd) -> bool {
    edos_rt::fd::isatty(fd.as_raw_fd() as u64)
}
