use edos_rt::FutexWaitResult;

use crate::sync::atomic::Atomic;
use crate::time::Duration;

/// An atomic for use as a futex that is at least 32-bits but may be larger
pub type Futex = Atomic<Primitive>;
/// Must be the underlying type of Futex
pub type Primitive = u32;

/// An atomic for use as a futex that is at least 8-bits but may be larger.
pub type SmallFutex = Atomic<SmallPrimitive>;
/// Must be the underlying type of SmallFutex
pub type SmallPrimitive = u32;

pub fn futex_wait(futex: &Futex, expected: u32, timeout: Option<Duration>) -> bool {
    let r = edos_rt::sync::futex_wait(futex, expected, timeout);

    // bool true means timeout?
    match r {
        Ok(result) => result != FutexWaitResult::TimedOut,
        Err(_) => false,
    }
}

#[inline]
pub fn futex_wake(futex: &Futex) -> bool {
    edos_rt::sync::futex_wake(futex, 1).is_ok()
}

#[inline]
pub fn futex_wake_all(futex: &Futex) {
    edos_rt::sync::futex_wake(futex, u32::MAX).unwrap();
}
