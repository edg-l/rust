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
use crate::path::{Path, PathBuf};
use crate::sys::fd::FileDesc;
use crate::sys::time::SystemTime;
use crate::sys::{cvt, cvt_io, unsupported};

pub struct File(pub(crate) FileDesc);

#[derive(Debug, Clone)]
pub struct FileAttr(FstatEntry);

pub struct ReadDir(!);

pub struct DirEntry(edos_rt::fs::DirEntry);

#[derive(Clone, Debug)]
pub struct OpenOptions {
    inner: edos_rt::fd::OpenFlags,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {}

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
        unsupported()
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        unsupported()
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        unsupported()
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
    pub fn set_accessed(&mut self, _t: SystemTime) {}
    pub fn set_modified(&mut self, _t: SystemTime) {}
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        matches!(self.0, edos_rt::fs::FileType::Directory)
    }

    pub fn is_file(&self) -> bool {
        matches!(self.0, edos_rt::fs::FileType::File)
    }

    pub fn is_symlink(&self) -> bool {
        false
    }
}

impl Hash for FileType {
    fn hash<H: Hasher>(&self, h: &mut H) {
        h.write_u64(self.0 as u64);
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        self.0
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.0.name.clone().into()
    }

    pub fn file_name(&self) -> OsString {
        self.0.name.clone().into()
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        unsupported()
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType(self.0.file_type))
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions { inner: edos_rt::fd::OpenFlags::Create }
    }

    pub fn read(&mut self, _read: bool) {}
    pub fn write(&mut self, _write: bool) {}
    pub fn append(&mut self, _append: bool) {
        self.inner = edos_rt::fd::OpenFlags::Append
    }
    pub fn truncate(&mut self, _truncate: bool) {}
    pub fn create(&mut self, _create: bool) {
        self.inner = edos_rt::fd::OpenFlags::Create
    }
    pub fn create_new(&mut self, _create_new: bool) {}
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        Ok(File(FileDesc {
            inner: cvt_io(edos_rt::fd::FileDesc::new(
                &path.as_os_str().to_string_lossy(),
                opts.inner,
            ))?,
        }))
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        let stat = self.0.fstat()?;
        Ok(FileAttr(stat))
    }

    pub fn fsync(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn datasync(&self) -> io::Result<()> {
        Ok(())
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

    pub fn truncate(&self, _size: u64) -> io::Result<()> {
        unsupported()
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
    pub fn read_buf(&self, buf: BorrowedCursor<'_>) -> io::Result<()> {
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
        unsupported()
    }

    pub fn duplicate(&self) -> io::Result<File> {
        unsupported()
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        unsupported()
    }

    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        unsupported()
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

pub fn readdir(_p: &Path) -> io::Result<ReadDir> {
    unsupported()
}

pub fn unlink(_p: &Path) -> io::Result<()> {
    unsupported()
}

pub fn rename(_old: &Path, _new: &Path) -> io::Result<()> {
    unsupported()
}

pub fn set_perm(_p: &Path, perm: FilePermissions) -> io::Result<()> {
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

pub fn exists(_path: &Path) -> io::Result<bool> {
    unsupported()
}

pub fn readlink(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    let attr = cvt_io(edos_rt::fd::stat_path(&p.to_string_lossy()))?;
    Ok(FileAttr(attr))
}

pub fn lstat(_p: &Path) -> io::Result<FileAttr> {
    unsupported()
}

pub fn canonicalize(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn copy(_from: &Path, _to: &Path) -> io::Result<u64> {
    unsupported()
}
