use edos_rt::process::thread_create;

use crate::io;
use crate::num::NonZero;
use crate::sys::error_kind;
use crate::thread::ThreadInit;
use crate::time::Duration;

pub struct Thread(u64);

pub const DEFAULT_MIN_STACK_SIZE: usize = 64 * 1024;

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(_stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        let p = Box::into_raw(init);

        let pid: io::Result<u64> = match thread_create(thread_start, p.cast()) {
            Ok(pid) => Ok(pid),
            Err(err) => Err(error_kind(err).into()),
        };

        Ok(Thread(pid?))
    }

    #[inline]
    pub fn join(self) {
        edos_rt::process::thread_join(self.0).unwrap();
    }
}

extern "C" fn thread_start(main: *mut u8) -> i32 {
    unsafe {
        let init = Box::from_raw(main.cast::<ThreadInit>());
        let rust_start = init.init();
        rust_start();

        // run all destructors
        crate::sys::thread_local::destructors::run();
        crate::rt::thread_cleanup();

        // Last, because everything above it can still free: the allocator
        // parks small blocks per thread, and a thread that exits holding them
        // strands them for the life of the process.
        crate::sys::alloc::flush_thread_cache();

        0
    }
}

/// The number of CPUs that came up, from `/proc/cpuinfo`.
///
/// The kernel reports both what it found in the ACPI tables and how many of
/// those actually started; the online count is the one a thread pool wants,
/// since an AP that failed to boot will run nothing.
pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    let info = crate::fs::read_to_string("/proc/cpuinfo")?;
    let online = info
        .lines()
        .find_map(|line| line.strip_prefix("cpus online:"))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .and_then(NonZero::new);

    online.ok_or(io::Error::UNKNOWN_THREAD_COUNT)
}

pub fn current_os_id() -> Option<u64> {
    Some(edos_rt::process::sys_getpid())
}

pub fn yield_now() {
    edos_rt::process::sched_yield();
}

pub fn sleep(dur: Duration) {
    // Nanoseconds, not milliseconds: rounding the request meant every sleep
    // shorter than a millisecond was either skipped or stretched to one.
    let secs = dur.as_secs().min(i64::MAX as u64) as i64;
    edos_rt::process::nanosleep(secs, dur.subsec_nanos() as i64).ok();
}
