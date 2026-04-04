use crate::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct SystemTime(Duration);

pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

impl Instant {
    pub fn now() -> Instant {
        let tick = edos_rt::process::monotonic_time_ns();
        Instant(Duration::from_nanos(tick))
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.checked_sub(other.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_sub(*other)?))
    }
}

impl SystemTime {
    pub const MAX: SystemTime = SystemTime(Duration::new(u64::MAX, 999_999_999));
    pub const MIN: SystemTime = SystemTime(Duration::from_secs(0));

    pub fn now() -> SystemTime {
        if let Some(t) = edos_rt::process::clock_gettime() {
            // Convert RTC to seconds since Unix epoch (approximate, no timezone)
            let year = t.year() as u64;
            // Days from 1970 to start of year (simplified, no leap second handling)
            let mut days: u64 = 0;
            for y in 1970..year {
                days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
            }
            let month_days: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            for m in 0..(t.month as u64).saturating_sub(1) {
                days += month_days[m as usize];
                if m == 1 && is_leap { days += 1; }
            }
            days += t.day.saturating_sub(1) as u64;
            let secs = days * 86400 + t.hour as u64 * 3600 + t.minute as u64 * 60 + t.second as u64;
            SystemTime(Duration::from_secs(secs))
        } else {
            UNIX_EPOCH
        }
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_sub(*other)?))
    }
}
