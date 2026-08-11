#![allow(unused)]

use core::str::FromStr;

use alloc_crate::ffi::CString;
use edos_rt::fd::{FstatEntry, OpenFlags};
use edos_rt::fs::{sys_mkdir, sys_rmdir, sys_rmdir_all};

use crate::ffi::OsString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::hash::{Hash, Hasher};
use crate::io::{self, BorrowedCursor, Error, IoSlice, IoSliceMut, SeekFrom};
use crate::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::path::{Path, PathBuf};
use crate::sys::fd::FileDesc;
pub use crate::sys::fs::common::Dir;
use crate::sys::time::{SystemTime, UNIX_EPOCH};
use crate::sys::{cvt, cvt_io, unsupported};
use crate::time::Duration;

pub struct File(pub(crate) FileDesc);

/// A `stat` timestamp, given as whole seconds since the Unix epoch.
///
/// Zero means the filesystem carries no such time. Returning the epoch for it
/// would date every file on such a volume to 1970, which is a wrong answer
/// rather than a missing one.
fn timestamp(secs: u64) -> io::Result<SystemTime> {
    if secs == 0 {
        return Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "this filesystem does not record that timestamp",
        ));
    }
    UNIX_EPOCH
        .checked_add_duration(&Duration::from_secs(secs))
        .ok_or_else(|| io::const_error!(io::ErrorKind::InvalidData, "timestamp out of range"))
}

#[derive(Debug, Clone)]
pub struct FileAttr(FstatEntry);

/// A directory walked in chunks.
///
/// `SYS_LIST_DIR` needs a buffer big enough for the whole directory at once, so
/// a large one either allocates for every entry up front or fails; `getdents`
/// takes a starting index, which is what lets this hold one chunk at a time.
#[derive(Debug)]
pub struct ReadDir {
    root: PathBuf,
    /// Decoded but not yet yielded, reversed so `pop` hands them out in order.
    batch: Vec<edos_rt::fs::DirEntry>,
    /// Entries already yielded, which is where the next chunk starts.
    consumed: usize,
    /// Set by the chunk that came back short, so the end costs no extra call.
    exhausted: bool,
}

#[derive(Debug)]
pub struct DirEntry {
    inner: edos_rt::fs::DirEntry,
    /// Directory this entry was read from. `path()` is documented to return a
    /// path usable on its own, so the entry has to remember where it came from.
    root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    inner: edos_rt::fd::OpenFlags,
    read: bool,
    write: bool,
}

/// Times to write, `None` meaning "leave this one alone".
///
/// The kernel stores whole seconds, so a `SystemTime` is truncated on the way
/// down; that is a property of the on-disk format rather than of this type,
/// which keeps what it was given.
#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilePermissions(u16);

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct FileType(edos_rt::fs::FileType);

#[derive(Debug)]
pub struct DirBuilder {}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.0.size
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions(0)
    }

    pub fn file_type(&self) -> FileType {
        FileType(edos_rt::fs::FileType::from(self.0.kind))
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        timestamp(self.0.modified)
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        timestamp(self.0.accessed)
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        timestamp(self.0.created)
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        false
    }

    pub fn set_readonly(&mut self, _readonly: bool) {}
}

impl Eq for FilePermissions {}

impl FileTimes {
    pub fn set_accessed(&mut self, t: SystemTime) {
        self.accessed = Some(t);
    }
    pub fn set_modified(&mut self, t: SystemTime) {
        self.modified = Some(t);
    }

    /// The pair the kernel expects, with `UTIME_OMIT` standing in for a time
    /// the caller did not set.
    fn pair(&self) -> io::Result<(edos_rt::fs::Timespec, edos_rt::fs::Timespec)> {
        let one = |t: Option<SystemTime>| -> io::Result<edos_rt::fs::Timespec> {
            match t {
                None => Ok(edos_rt::fs::Timespec::OMIT),
                Some(t) => {
                    let secs = t
                        .sub_time(&UNIX_EPOCH)
                        .map_err(|_| {
                            io::const_error!(
                                io::ErrorKind::InvalidInput,
                                "file times before the Unix epoch cannot be stored"
                            )
                        })?
                        .as_secs();
                    Ok(edos_rt::fs::Timespec { tv_sec: secs as i64, tv_nsec: 0 })
                }
            }
        };
        Ok((one(self.accessed)?, one(self.modified)?))
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        matches!(self.0, edos_rt::fs::FileType::Directory)
    }

    pub fn is_file(&self) -> bool {
        matches!(self.0, edos_rt::fs::FileType::File)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self.0, edos_rt::fs::FileType::Symlink)
    }
}

impl Hash for FileType {
    fn hash<H: Hasher>(&self, h: &mut H) {
        h.write_u64(self.0 as u64);
    }
}

impl ReadDir {
    fn new(root: PathBuf, first: Vec<edos_rt::fs::DirEntry>) -> ReadDir {
        let mut dir = ReadDir { root, batch: Vec::new(), consumed: 0, exhausted: false };
        dir.fill(first);
        dir
    }

    fn fill(&mut self, mut entries: Vec<edos_rt::fs::DirEntry>) {
        self.exhausted = entries.is_empty();
        entries.reverse();
        self.batch = entries;
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        if self.batch.is_empty() {
            if self.exhausted {
                return None;
            }
            match edos_rt::fs::read_dir_from(&self.root.to_string_lossy(), self.consumed) {
                Ok(entries) => self.fill(entries),
                Err(e) => return Some(Err(io::Error::from_raw_os_error(e as i32))),
            }
            if self.batch.is_empty() {
                return None;
            }
        }
        let inner = self.batch.pop()?;
        self.consumed += 1;
        Some(Ok(DirEntry { inner, root: self.root.clone() }))
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.root.join(&self.inner.name)
    }

    pub fn file_name(&self) -> OsString {
        self.inner.name.clone().into()
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        stat(&self.path())
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType(self.inner.file_type))
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions { inner: edos_rt::fd::OpenFlags::NONE, read: false, write: false }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        if append {
            // Appending implies writing, which matters because the access mode
            // is what the kernel checks before permitting a shared writable
            // mapping or a write syscall.
            self.write = true;
            self.inner |= edos_rt::fd::OpenFlags::APPEND;
        }
    }
    pub fn truncate(&mut self, truncate: bool) {
        if truncate {
            self.inner |= edos_rt::fd::OpenFlags::TRUNCATE;
        }
    }
    pub fn create(&mut self, create: bool) {
        if create {
            self.inner |= edos_rt::fd::OpenFlags::CREATE;
        }
    }
    pub fn create_new(&mut self, create_new: bool) {
        if create_new {
            self.inner |= edos_rt::fd::OpenFlags::CREATE;
        }
    }

    fn access_flags(&self) -> edos_rt::fd::OpenFlags {
        use edos_rt::fd::OpenFlags;
        match (self.read, self.write) {
            (true, true) => OpenFlags::READ_WRITE,
            (false, true) => OpenFlags::WRITE_ONLY,
            _ => OpenFlags::READ_ONLY,
        }
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        Ok(File(FileDesc {
            inner: cvt_io(edos_rt::fd::FileDesc::new(
                &path.as_os_str().to_string_lossy(),
                opts.inner | opts.access_flags(),
            ))?,
        }))
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        let stat = self.0.fstat()?;
        Ok(FileAttr(stat))
    }

    pub fn fsync(&self) -> io::Result<()> {
        self.0.fsync()
    }

    /// The kernel has no `fdatasync`: `SYS_FSYNC` already flushes data and
    /// metadata together, so this is `fsync` rather than a weaker guarantee.
    pub fn datasync(&self) -> io::Result<()> {
        self.0.fsync()
    }

    pub fn lock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(Error::UNSUPPORTED_PLATFORM))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(Error::UNSUPPORTED_PLATFORM))
    }

    pub fn unlock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn truncate(&self, size: u64) -> io::Result<()> {
        cvt_io(self.0.inner.ftruncate(size))
    }

    #[inline]
    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    #[inline]
    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }

    #[inline]
    pub fn is_read_vectored(&self) -> bool {
        self.0.is_read_vectored()
    }

    #[inline]
    pub fn read_buf(&self, buf: BorrowedCursor<'_, u8>) -> io::Result<()> {
        self.0.read_buf(buf)
    }

    #[inline]
    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        self.0.is_write_vectored()
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        Some(self.0.fstat().map(|x| x.size))
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.0.tell()
    }

    pub fn duplicate(&self) -> io::Result<File> {
        Ok(File(self.0.try_clone()?))
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        unsupported()
    }

    pub fn set_times(&self, times: FileTimes) -> io::Result<()> {
        let (accessed, modified) = times.pair()?;
        cvt_io(edos_rt::fs::set_fd_times(self.0.as_raw_fd() as u64, accessed, modified))
    }

    pub fn from_raw_fd(fd: u64, flags: OpenFlags) -> Self {
        Self(FileDesc { inner: edos_rt::fd::FileDesc::from_raw_fd(fd, flags) })
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, p: &Path) -> io::Result<()> {
        let cstr = CString::from_str(&p.as_os_str().to_string_lossy()).unwrap();
        cvt(unsafe { sys_mkdir(cstr.as_ptr().cast()) as isize })?;
        Ok(())
    }
}

impl fmt::Debug for File {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    // The first chunk is read here rather than on the first `next()`: a missing
    // or unreadable directory is `read_dir`'s error to report, and an iterator
    // that only fails once it is walked reports it to nobody.
    let first = cvt_io(edos_rt::fs::read_dir_from(&p.to_string_lossy(), 0))?;
    Ok(ReadDir::new(p.to_path_buf(), first))
}

pub fn unlink(p: &Path) -> io::Result<()> {
    cvt_io(edos_rt::fs::unlink(&p.to_string_lossy()))?;
    Ok(())
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    cvt_io(edos_rt::fs::rename(&old.to_string_lossy(), &new.to_string_lossy()))
}

pub fn set_perm(_p: &Path, perm: FilePermissions) -> io::Result<()> {
    match perm.0 {
        _ => Ok(()),
    }
}

pub fn set_perm_nofollow(_p: &Path, perm: FilePermissions) -> io::Result<()> {
    match perm.0 {
        _ => Ok(()),
    }
}

pub fn rmdir(p: &Path) -> io::Result<()> {
    let cstr = CString::from_str(&p.as_os_str().to_string_lossy()).unwrap();
    cvt(unsafe { sys_rmdir(cstr.as_ptr().cast()) as isize })?;
    Ok(())
}

pub fn remove_dir_all(p: &Path) -> io::Result<()> {
    let cstr = CString::from_str(&p.as_os_str().to_string_lossy()).unwrap();
    cvt(unsafe { sys_rmdir_all(cstr.as_ptr().cast()) as isize })?;
    Ok(())
}

pub fn exists(path: &Path) -> io::Result<bool> {
    // `access` with `F_OK`, which asks the question directly rather than
    // building a whole `stat` and throwing it away.
    Ok(edos_rt::fs::access(&path.to_string_lossy(), edos_rt::sys::F_OK))
}

pub fn readlink(p: &Path) -> io::Result<PathBuf> {
    let target = cvt_io(edos_rt::fs::read_link(&p.to_string_lossy()))?;
    Ok(PathBuf::from(target))
}

pub fn symlink(original: &Path, link: &Path) -> io::Result<()> {
    cvt_io(edos_rt::fs::symlink(&original.to_string_lossy(), &link.to_string_lossy()))
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    let attr = cvt_io(edos_rt::fd::stat_path(&p.to_string_lossy()))?;
    Ok(FileAttr(attr))
}

pub fn lstat(p: &Path) -> io::Result<FileAttr> {
    let attr = cvt_io(edos_rt::fd::lstat_path(&p.to_string_lossy()))?;
    Ok(FileAttr(attr))
}

pub fn canonicalize(p: &Path) -> io::Result<PathBuf> {
    let s = p.to_string_lossy();
    if s.starts_with('/') {
        Ok(p.to_path_buf())
    } else {
        let cwd = cvt_io(edos_rt::fs::getcwd())?;
        let mut full = PathBuf::from(cwd);
        full.push(p);
        Ok(full)
    }
}

pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    let reader = File::open(from, &OpenOptions::new())?;
    let writer = File::open(to, &{
        let mut opts = OpenOptions::new();
        opts.create(true);
        opts
    })?;
    let mut buf = [0u8; 4096];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write(&buf[..n])?;
        total += n as u64;
    }
    Ok(total)
}

pub fn set_times(p: &Path, times: FileTimes) -> io::Result<()> {
    let (accessed, modified) = times.pair()?;
    cvt_io(edos_rt::fs::set_times(&p.to_string_lossy(), accessed, modified))
}

pub fn set_times_nofollow(_p: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

impl AsRawFd for File {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl IntoRawFd for File {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        self.0.into_raw_fd()
    }
}

impl FromRawFd for File {
    #[inline]
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        File(unsafe { FileDesc::from_raw_fd(fd) })
    }
}

impl crate::sys::AsInner<FileDesc> for File {
    fn as_inner(&self) -> &FileDesc {
        &self.0
    }
}

impl crate::sys::IntoInner<FileDesc> for File {
    fn into_inner(self) -> FileDesc {
        self.0
    }
}

impl crate::sys::FromInner<FileDesc> for File {
    fn from_inner(fd: FileDesc) -> Self {
        File(fd)
    }
}

impl File {
    pub fn as_fd(&self) -> crate::os::fd::BorrowedFd<'_> {
        self.0.as_fd()
    }
}
