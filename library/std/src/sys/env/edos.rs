pub use super::common::Env;
use crate::ffi::{OsStr, OsString};
use crate::io;
use crate::os::edos::ffi::{OsStrExt, OsStringExt};

pub fn env() -> Env {
    let vars: Vec<(OsString, OsString)> = edos_rt::env::env_iter()
        .into_iter()
        .map(|(k, v)| (OsString::from_vec(k), OsString::from_vec(v)))
        .collect();
    Env::new(vars)
}

pub fn getenv(k: &OsStr) -> Option<OsString> {
    edos_rt::env::get_env(k.as_bytes()).map(OsString::from_vec)
}

pub unsafe fn setenv(k: &OsStr, v: &OsStr) -> io::Result<()> {
    edos_rt::env::set_env(k.as_bytes(), v.as_bytes());
    Ok(())
}

pub unsafe fn unsetenv(k: &OsStr) -> io::Result<()> {
    edos_rt::env::unset_env(k.as_bytes());
    Ok(())
}
