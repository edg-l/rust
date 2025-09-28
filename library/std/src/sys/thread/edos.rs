use core::ptr::null_mut;

use crate::io;
use crate::num::NonZero;
use crate::time::Duration;

pub struct Thread(u64);

pub const DEFAULT_MIN_STACK_SIZE: usize = 64 * 1024;

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(
        _stack: usize,
        _name: Option<&str>,
        p: Box<dyn FnOnce()>,
    ) -> io::Result<Thread> {
        let p = Box::into_raw(Box::new(p));
        let pid = edos_rt::process::sys_clone(thread_start, p.cast(), 0, null_mut());

        Ok(Thread(pid))
    }

    #[inline]
    pub fn join(self) {
        edos_rt::process::thread_join(self.0).unwrap();
    }
}

extern "C" fn thread_start(main: *mut u8) -> i32 {
    unsafe {
        // Finally, let's run some code.
        Box::from_raw(main.cast::<Box<dyn FnOnce()>>())();

        // run all destructors
        crate::sys::thread_local::destructors::run();
        crate::rt::thread_cleanup();

        0
    }
}

pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    Err(io::Error::UNKNOWN_THREAD_COUNT)
}

pub fn current_os_id() -> Option<u64> {
    Some(edos_rt::process::sys_getpid())
}

pub fn yield_now() {
    // do nothing
}

pub fn sleep(dur: Duration) {
    let mut ms = dur.as_millis();

    if ms > u64::MAX as u128 {
        ms = u64::MAX as u128;
    }
    edos_rt::process::sleep_ms(ms as u64).ok();
}
