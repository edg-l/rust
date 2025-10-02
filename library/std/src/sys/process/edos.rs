use alloc_crate::ffi::CString;
use edos_rt::io::cvt;

use super::env::{CommandEnv, CommandEnvs};
use crate::env::current_dir;
pub use crate::ffi::OsString as EnvKey;
use crate::ffi::{OsStr, OsString};
use crate::num::NonZero;
use crate::path::Path;
use crate::process::StdioPipes;
use crate::sys::fd::FileDesc;
use crate::sys::fs::File;
use crate::sys::pipe::AnonPipe;
use crate::sys::{cvt_io, decode_error_kind, unsupported};
use crate::sys_common::FromInner;
use crate::{fmt, io};

////////////////////////////////////////////////////////////////////////////////
// Command
////////////////////////////////////////////////////////////////////////////////

pub struct Command {
    program: OsString,
    args: Vec<OsString>,
    env: CommandEnv,

    cwd: Option<OsString>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

#[derive(Debug)]
pub enum Stdio {
    Inherit,
    Null,
    MakePipe,
    ParentStdout,
    ParentStderr,
    #[allow(dead_code)] // This variant exists only for the Debug impl
    InheritFile(File),
}

impl Command {
    pub fn new(program: &OsStr) -> Command {
        Command {
            program: program.to_owned(),
            args: vec![program.to_owned()],
            env: Default::default(),
            cwd: current_dir().ok().map(|x| x.into_os_string()),
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    pub fn arg(&mut self, arg: &OsStr) {
        self.args.push(arg.to_owned());
    }

    pub fn env_mut(&mut self) -> &mut CommandEnv {
        &mut self.env
    }

    pub fn cwd(&mut self, dir: &OsStr) {
        self.cwd = Some(dir.to_owned());
    }

    pub fn stdin(&mut self, stdin: Stdio) {
        self.stdin = Some(stdin);
    }

    pub fn stdout(&mut self, stdout: Stdio) {
        self.stdout = Some(stdout);
    }

    pub fn stderr(&mut self, stderr: Stdio) {
        self.stderr = Some(stderr);
    }

    pub fn get_program(&self) -> &OsStr {
        &self.program
    }

    pub fn get_args(&self) -> CommandArgs<'_> {
        let mut iter = self.args.iter();
        iter.next();
        CommandArgs { iter }
    }

    pub fn get_envs(&self) -> CommandEnvs<'_> {
        self.env.iter()
    }

    pub fn get_current_dir(&self) -> Option<&Path> {
        self.cwd.as_ref().map(|cs| Path::new(cs))
    }

    pub fn spawn(
        &mut self,
        default: Stdio,
        _needs_stdin: bool,
    ) -> io::Result<(Process, StdioPipes)> {
        let program = CString::new(self.program.to_string_lossy().to_string()).unwrap();
        let args = &self.args;

        let mut argv_storage: Vec<Vec<u8>> = Vec::with_capacity(args.len());
        let mut argv_ptrs: Vec<*const u8> = Vec::with_capacity(args.len() + 1);

        for arg in args.iter() {
            let mut buf = Vec::with_capacity(arg.len() + 1);
            buf.extend_from_slice(arg.as_encoded_bytes());
            buf.push(0);
            argv_ptrs.push(buf.as_ptr());
            argv_storage.push(buf);
        }

        // Null terminate
        argv_ptrs.push(core::ptr::null());

        let argv_ptr = if argv_ptrs.is_empty() { core::ptr::null() } else { argv_ptrs.as_ptr() };

        let mut pipes = [const { None }; 3];

        let stdin = match self.stdin.as_ref().unwrap_or(&default) {
            Stdio::Inherit => 0,
            Stdio::Null => unsupported()?,
            Stdio::MakePipe => {
                let (read_fd, write_fd) = edos_rt::process::pipe().unwrap();

                pipes[0] = Some(AnonPipe::from_inner(FileDesc {
                    inner: edos_rt::fd::FileDesc::from_raw_fd(
                        write_fd,
                        edos_rt::fd::OpenFlags::Create,
                    ),
                }));

                read_fd
            }
            Stdio::ParentStdout => 1,
            Stdio::ParentStderr => 2,
            Stdio::InheritFile(file) => file.0.inner.raw_fd(),
        };

        let stdout = match self.stdin.as_ref().unwrap_or(&default) {
            Stdio::Inherit => 1,
            Stdio::Null => unsupported()?,
            Stdio::MakePipe => {
                let (read_fd, write_fd) = edos_rt::process::pipe().unwrap();

                pipes[1] = Some(AnonPipe::from_inner(FileDesc {
                    inner: edos_rt::fd::FileDesc::from_raw_fd(
                        read_fd,
                        edos_rt::fd::OpenFlags::Create,
                    ),
                }));

                write_fd
            }
            Stdio::ParentStdout => 1,
            Stdio::ParentStderr => 2,
            Stdio::InheritFile(file) => file.0.inner.raw_fd(),
        };

        let stderr = match self.stdin.as_ref().unwrap_or(&default) {
            Stdio::Inherit => 2,
            Stdio::Null => unsupported()?,
            Stdio::MakePipe => {
                let (read_fd, write_fd) = edos_rt::process::pipe().unwrap();

                pipes[2] = Some(AnonPipe::from_inner(FileDesc {
                    inner: edos_rt::fd::FileDesc::from_raw_fd(
                        read_fd,
                        edos_rt::fd::OpenFlags::Create,
                    ),
                }));

                write_fd
            }
            Stdio::ParentStdout => 1,
            Stdio::ParentStderr => 2,
            Stdio::InheritFile(file) => file.0.inner.raw_fd(),
        };

        let result = cvt_io(cvt(unsafe {
            edos_rt::sys_spawn(
                program.as_ptr().cast(),
                argv_ptr.cast_mut().cast(),
                stdin,
                stdout,
                stderr,
            )
        } as isize))?;
        Ok((
            Process(result as u64),
            StdioPipes { stderr: pipes[2].take(), stdout: pipes[1].take(), stdin: pipes[0].take() },
        ))
    }
}

pub fn output(_cmd: &mut Command) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
    unsupported()
}

impl From<AnonPipe> for Stdio {
    fn from(pipe: AnonPipe) -> Stdio {
        Stdio::InheritFile(File::from_raw_fd(
            pipe.into_raw_fd().inner.raw_fd(),
            edos_rt::fd::OpenFlags::Create,
        ))
    }
}

impl From<io::Stdout> for Stdio {
    fn from(_: io::Stdout) -> Stdio {
        Stdio::ParentStdout
    }
}

impl From<io::Stderr> for Stdio {
    fn from(_: io::Stderr) -> Stdio {
        Stdio::ParentStderr
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Stdio {
        Stdio::InheritFile(file)
    }
}

impl fmt::Debug for Command {
    // show all attributes
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let mut debug_command = f.debug_struct("Command");
            debug_command.field("program", &self.program).field("args", &self.args);
            if !self.env.is_unchanged() {
                debug_command.field("env", &self.env);
            }

            if self.cwd.is_some() {
                debug_command.field("cwd", &self.cwd);
            }

            if self.stdin.is_some() {
                debug_command.field("stdin", &self.stdin);
            }
            if self.stdout.is_some() {
                debug_command.field("stdout", &self.stdout);
            }
            if self.stderr.is_some() {
                debug_command.field("stderr", &self.stderr);
            }

            debug_command.finish()
        } else {
            if let Some(ref cwd) = self.cwd {
                write!(f, "cd {cwd:?} && ")?;
            }
            if self.env.does_clear() {
                write!(f, "env -i ")?;
                // Altered env vars will be printed next, that should exactly work as expected.
            } else {
                // Removed env vars need the command to be wrapped in `env`.
                let mut any_removed = false;
                for (key, value_opt) in self.get_envs() {
                    if value_opt.is_none() {
                        if !any_removed {
                            write!(f, "env ")?;
                            any_removed = true;
                        }
                        write!(f, "-u {} ", key.to_string_lossy())?;
                    }
                }
            }
            // Altered env vars can just be added in front of the program.
            for (key, value_opt) in self.get_envs() {
                if let Some(value) = value_opt {
                    write!(f, "{}={value:?} ", key.to_string_lossy())?;
                }
            }
            if self.program != self.args[0] {
                write!(f, "[{:?}] ", self.program)?;
            }
            write!(f, "{:?}", self.args[0])?;

            for arg in &self.args[1..] {
                write!(f, " {:?}", arg)?;
            }
            Ok(())
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct ExitStatus(i32);

impl ExitStatus {
    pub fn exit_ok(&self) -> Result<(), ExitStatusError> {
        Ok(())
    }

    pub fn code(&self) -> Option<i32> {
        Some(0)
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<dummy exit status>")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExitStatusError(i32);

impl ExitStatusError {
    pub fn code(self) -> Option<NonZero<i32>> {
        NonZero::new(self.0)
    }
}

impl From<ExitStatusError> for ExitStatus {
    fn from(value: ExitStatusError) -> Self {
        ExitStatus(value.0)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitCode(u8);

impl ExitCode {
    pub const SUCCESS: ExitCode = ExitCode(0);
    pub const FAILURE: ExitCode = ExitCode(1);

    pub fn as_i32(&self) -> i32 {
        self.0 as i32
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        Self(code)
    }
}

pub struct Process(u64);

impl Process {
    pub fn id(&self) -> u32 {
        self.0 as u32
    }

    pub fn kill(&mut self) -> io::Result<()> {
        unsupported()
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        loop {
            let result = edos_rt::process::sys_waitpid(self.0, true);

            match result {
                Ok(status) => match status {
                    edos_rt::process::WaitPidStatus::StillRunning => continue,
                    edos_rt::process::WaitPidStatus::Exited(code) => return Ok(ExitStatus(code)),
                },
                Err(err) => return Err(decode_error_kind(err as i32).into()),
            }
        }
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let result = edos_rt::process::sys_waitpid(self.0, true);

        match result {
            Ok(status) => match status {
                edos_rt::process::WaitPidStatus::StillRunning => Ok(None),
                edos_rt::process::WaitPidStatus::Exited(code) => return Ok(Some(ExitStatus(code))),
            },
            Err(err) => return Err(decode_error_kind(err as i32).into()),
        }
    }
}

pub struct CommandArgs<'a> {
    iter: crate::slice::Iter<'a, OsString>,
}

impl<'a> Iterator for CommandArgs<'a> {
    type Item = &'a OsStr;
    fn next(&mut self) -> Option<&'a OsStr> {
        self.iter.next().map(|os| &**os)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for CommandArgs<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
    fn is_empty(&self) -> bool {
        self.iter.is_empty()
    }
}

impl<'a> fmt::Debug for CommandArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter.clone()).finish()
    }
}
