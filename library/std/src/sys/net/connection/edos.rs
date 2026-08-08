//! Sockets for edos, over the kernel's socket syscalls.
//!
//! The kernel speaks IPv4 only, so every entry point that takes a
//! [`SocketAddr`] rejects a v6 one rather than silently truncating it.

use edos_rt::net::{
    self as rt, DnsError, IP_TTL, IPPROTO_IP, IPPROTO_TCP, SO_ERROR, SO_LINGER, SO_RCVTIMEO,
    SO_SNDTIMEO, SOL_SOCKET, SockAddrIn, TCP_NODELAY,
};

use crate::fmt;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, ToSocketAddrs};
use crate::sys::net::connection::each_addr;
use crate::sys::{error_kind, unsupported};
use crate::time::Duration;
use crate::vec;

/// `struct timeval`, the layout the kernel reads for the timeout options.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Timeval {
    sec: i64,
    usec: i64,
}

/// `struct linger`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Linger {
    onoff: i32,
    linger: i32,
}

fn err(e: edos_rt::sys::Errno) -> io::Error {
    io::Error::from(error_kind(e))
}

fn last_error() -> io::Error {
    err(edos_rt::sys::errno())
}

fn cvt(ret: i64) -> io::Result<i64> {
    if ret < 0 { Err(last_error()) } else { Ok(ret) }
}

fn dns_error(e: DnsError) -> io::Error {
    match e {
        DnsError::Io(errno) => err(errno),
        DnsError::Rcode(3) | DnsError::NoAddress => {
            io::const_error!(io::ErrorKind::NotFound, "no A record for host")
        }
        DnsError::Truncated => {
            io::const_error!(io::ErrorKind::InvalidData, "DNS response truncated")
        }
        DnsError::Malformed => {
            io::const_error!(io::ErrorKind::InvalidData, "malformed DNS response")
        }
        DnsError::Rcode(_) => {
            io::const_error!(io::ErrorKind::Other, "DNS server returned an error")
        }
    }
}

fn to_sockaddr(addr: &SocketAddr) -> io::Result<SockAddrIn> {
    match addr {
        SocketAddr::V4(v4) => Ok(SockAddrIn {
            family: rt::AF_INET as u16,
            port: v4.port().to_be(),
            addr: v4.ip().octets(),
            zero: [0; 8],
        }),
        SocketAddr::V6(_) => Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "IPv6 is not implemented on this platform"
        )),
    }
}

fn from_sockaddr(addr: &SockAddrIn) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(addr.addr), u16::from_be(addr.port)))
}

/// Owns a socket descriptor and closes it on drop.
struct Socket(u64);

impl Socket {
    fn new(sock_type: u32) -> io::Result<Socket> {
        let fd = cvt(rt::sys_socket(rt::AF_INET, sock_type, 0))?;
        Ok(Socket(fd as u64))
    }

    fn from_raw(fd: u64) -> Socket {
        Socket(fd)
    }

    fn raw(&self) -> u64 {
        self.0
    }

    fn duplicate(&self) -> io::Result<Socket> {
        let fd = edos_rt::fd::sys_dup(self.0);
        if fd == u64::MAX { Err(last_error()) } else { Ok(Socket(fd)) }
    }

    fn setsockopt<T>(&self, level: i32, name: i32, value: T) -> io::Result<()> {
        let ret = rt::sys_setsockopt(
            self.0,
            level,
            name,
            &value as *const T as *const u8,
            size_of::<T>() as u32,
        );
        cvt(ret).map(drop)
    }

    fn getsockopt<T: Default>(&self, level: i32, name: i32) -> io::Result<T> {
        let mut value = T::default();
        let mut len = size_of::<T>() as u32;
        let ret = rt::sys_getsockopt(
            self.0,
            level,
            name,
            &mut value as *mut T as *mut u8,
            &mut len as *mut u32,
        );
        cvt(ret).map(|_| value)
    }

    fn set_timeout(&self, timeout: Option<Duration>, name: i32) -> io::Result<()> {
        let tv = match timeout {
            Some(d) if d == Duration::ZERO => {
                return Err(io::const_error!(
                    io::ErrorKind::InvalidInput,
                    "cannot set a 0 duration timeout"
                ));
            }
            Some(d) => Timeval { sec: d.as_secs() as i64, usec: d.subsec_micros() as i64 },
            None => Timeval { sec: 0, usec: 0 },
        };
        self.setsockopt(SOL_SOCKET, name, tv)
    }

    fn timeout(&self, name: i32) -> io::Result<Option<Duration>> {
        let tv: Timeval = self.getsockopt(SOL_SOCKET, name)?;
        if tv.sec == 0 && tv.usec == 0 {
            return Ok(None);
        }
        Ok(Some(Duration::new(tv.sec as u64, (tv.usec * 1000) as u32)))
    }

    fn peer_addr(&self) -> io::Result<SocketAddr> {
        let mut addr = SockAddrIn { family: 0, port: 0, addr: [0; 4], zero: [0; 8] };
        let mut len = size_of::<SockAddrIn>() as u32;
        cvt(rt::sys_getpeername(self.0, &mut addr, &mut len))?;
        Ok(from_sockaddr(&addr))
    }

    fn socket_addr(&self) -> io::Result<SocketAddr> {
        let mut addr = SockAddrIn { family: 0, port: 0, addr: [0; 4], zero: [0; 8] };
        let mut len = size_of::<SockAddrIn>() as u32;
        cvt(rt::sys_getsockname(self.0, &mut addr, &mut len))?;
        Ok(from_sockaddr(&addr))
    }

    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe { edos_rt::io::sys_read(self.0, buf.as_mut_ptr(), buf.len()) };
        if n < 0 { Err(last_error()) } else { Ok(n as usize) }
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let n = unsafe { edos_rt::io::sys_write(self.0, buf.as_ptr(), buf.len()) };
        if n < 0 { Err(last_error()) } else { Ok(n as usize) }
    }

    fn take_error(&self) -> io::Result<Option<io::Error>> {
        let code: i32 = self.getsockopt(SOL_SOCKET, SO_ERROR)?;
        Ok((code != 0).then(|| io::Error::from_raw_os_error(code)))
    }

    fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.setsockopt(IPPROTO_IP, IP_TTL, ttl as i32)
    }

    fn ttl(&self) -> io::Result<u32> {
        let ttl: i32 = self.getsockopt(IPPROTO_IP, IP_TTL)?;
        Ok(ttl as u32)
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        edos_rt::fd::sys_close(self.0);
    }
}

pub struct TcpStream(Socket);

impl TcpStream {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        each_addr(addr, |addr| {
            let sock = Socket::new(rt::SOCK_STREAM)?;
            cvt(rt::sys_connect(sock.raw(), &to_sockaddr(addr)?))?;
            Ok(TcpStream(sock))
        })
    }

    /// The kernel's connect blocks for its own bounded wait, and there is no
    /// non-blocking connect to drive a deadline from.
    pub fn connect_timeout(_: &SocketAddr, _: Duration) -> io::Result<TcpStream> {
        unsupported()
    }

    pub fn set_read_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(t, SO_RCVTIMEO)
    }

    pub fn set_write_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(t, SO_SNDTIMEO)
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(SO_RCVTIMEO)
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(SO_SNDTIMEO)
    }

    /// No `MSG_PEEK` in the kernel's `recvfrom`.
    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    pub fn read_buf(&self, mut cursor: BorrowedCursor<'_>) -> io::Result<()> {
        // SAFETY: `read` only writes to the buffer, and is told exactly how
        // many bytes it may write; `advance_unchecked` is then given the count
        // it reported, so only initialised bytes are ever exposed.
        unsafe {
            let n = self.0.read(cursor.as_mut().assume_init_mut())?;
            cursor.advance_unchecked(n);
        }
        Ok(())
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        match bufs.iter_mut().find(|b| !b.is_empty()) {
            Some(buf) => self.0.read(buf),
            None => Ok(0),
        }
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        match bufs.iter().find(|b| !b.is_empty()) {
            Some(buf) => self.0.write(buf),
            None => Ok(0),
        }
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        let how = match how {
            Shutdown::Read => 0,
            Shutdown::Write => 1,
            Shutdown::Both => 2,
        };
        cvt(rt::sys_shutdown(self.0.raw(), how)).map(drop)
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        self.0.duplicate().map(TcpStream)
    }

    pub fn set_linger(&self, linger: Option<Duration>) -> io::Result<()> {
        let l = Linger {
            onoff: linger.is_some() as i32,
            linger: linger.map(|d| d.as_secs() as i32).unwrap_or_default(),
        };
        self.0.setsockopt(SOL_SOCKET, SO_LINGER, l)
    }

    pub fn linger(&self) -> io::Result<Option<Duration>> {
        let l: Linger = self.0.getsockopt(SOL_SOCKET, SO_LINGER)?;
        Ok((l.onoff != 0).then(|| Duration::from_secs(l.linger as u64)))
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.0.setsockopt(IPPROTO_TCP, TCP_NODELAY, nodelay as i32)
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        let v: i32 = self.0.getsockopt(IPPROTO_TCP, TCP_NODELAY)?;
        Ok(v != 0)
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.set_ttl(ttl)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.0.ttl()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    /// The kernel has no `O_NONBLOCK`; every socket call blocks.
    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("TcpStream");
        b.field("fd", &self.0.raw());
        if let Ok(addr) = self.socket_addr() {
            b.field("local", &addr);
        }
        if let Ok(peer) = self.peer_addr() {
            b.field("peer", &peer);
        }
        b.finish()
    }
}

pub struct TcpListener(Socket);

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        each_addr(addr, |addr| {
            let sock = Socket::new(rt::SOCK_STREAM)?;
            cvt(rt::sys_bind(sock.raw(), &to_sockaddr(addr)?))?;
            cvt(rt::sys_listen(sock.raw(), 128))?;
            Ok(TcpListener(sock))
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let mut peer = SockAddrIn { family: 0, port: 0, addr: [0; 4], zero: [0; 8] };
        let fd = cvt(rt::sys_accept(self.0.raw(), Some(&mut peer)))?;
        Ok((TcpStream(Socket::from_raw(fd as u64)), from_sockaddr(&peer)))
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        self.0.duplicate().map(TcpListener)
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.set_ttl(ttl)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.0.ttl()
    }

    pub fn set_only_v6(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn only_v6(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("TcpListener");
        b.field("fd", &self.0.raw());
        if let Ok(addr) = self.socket_addr() {
            b.field("local", &addr);
        }
        b.finish()
    }
}

pub struct UdpSocket(Socket);

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket> {
        each_addr(addr, |addr| {
            let sock = Socket::new(rt::SOCK_DGRAM)?;
            cvt(rt::sys_bind(sock.raw(), &to_sockaddr(addr)?))?;
            Ok(UdpSocket(sock))
        })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let mut from = SockAddrIn { family: 0, port: 0, addr: [0; 4], zero: [0; 8] };
        let n = cvt(rt::sys_recvfrom(self.0.raw(), buf, 0, Some(&mut from)))?;
        Ok((n as usize, from_sockaddr(&from)))
    }

    pub fn peek_from(&self, _: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        unsupported()
    }

    pub fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> io::Result<usize> {
        let n = cvt(rt::sys_sendto(self.0.raw(), buf, 0, Some(&to_sockaddr(addr)?)))?;
        Ok(n as usize)
    }

    pub fn duplicate(&self) -> io::Result<UdpSocket> {
        self.0.duplicate().map(UdpSocket)
    }

    pub fn set_read_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(t, SO_RCVTIMEO)
    }

    pub fn set_write_timeout(&self, t: Option<Duration>) -> io::Result<()> {
        self.0.set_timeout(t, SO_SNDTIMEO)
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(SO_RCVTIMEO)
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        self.0.timeout(SO_SNDTIMEO)
    }

    pub fn set_broadcast(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn broadcast(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_multicast_loop_v4(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_multicast_ttl_v4(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn set_multicast_loop_v6(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn join_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        unsupported()
    }

    pub fn join_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn leave_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        unsupported()
    }

    pub fn leave_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.0.set_ttl(ttl)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.0.ttl()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let n = cvt(rt::sys_recvfrom(self.0.raw(), buf, 0, None))?;
        Ok(n as usize)
    }

    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let n = cvt(rt::sys_sendto(self.0.raw(), buf, 0, None))?;
        Ok(n as usize)
    }

    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        each_addr(addr, |addr| cvt(rt::sys_connect(self.0.raw(), &to_sockaddr(addr)?)).map(drop))
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("UdpSocket");
        b.field("fd", &self.0.raw());
        if let Ok(addr) = self.socket_addr() {
            b.field("local", &addr);
        }
        b.finish()
    }
}

/// One address per host: the resolver returns the first A record it finds.
pub struct LookupHost(vec::IntoIter<SocketAddr>);

impl Iterator for LookupHost {
    type Item = SocketAddr;
    fn next(&mut self) -> Option<SocketAddr> {
        self.0.next()
    }
}

pub fn lookup_host(host: &str, port: u16) -> io::Result<LookupHost> {
    let ip = rt::lookup_a(host).map_err(dns_error)?;
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(ip), port));
    Ok(LookupHost(vec![addr].into_iter()))
}
