//! In-process clock for managed-layer GC bounds that must count host suspend.
//!
//! `std::time::Instant` and Tokio's clock are `CLOCK_MONOTONIC` on Linux,
//! which stops while the host is suspended or hibernated. Protocol bounds
//! measured inside one process (the GC's delete phase and intent hold, a
//! node's lease freshness, tail and shrink debounce) use `CLOCK_BOOTTIME`
//! instead, which keeps counting through suspend. Waits keep Tokio sleeps: a
//! stopped clock only lengthens them in real time.
//!
//! No in-guest clock sees a hypervisor pause of a VM-hosted process; see
//! `docs/src/internals/snapshot-layer-gc.md` (Clocks).
//!
//! Non-Linux builds fall back to `std::time::Instant`; they are not a
//! production target. Under `cfg(test)` the clock follows Tokio's pausable
//! clock plus a per-thread offset that simulates host suspend
//! ([`advance_boot_clock_for_test`]).

use std::time::Duration;

/// A point on the boot clock.
#[cfg(not(test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BootInstant(Duration);

#[cfg(not(test))]
impl BootInstant {
    pub(crate) fn now() -> Self {
        Self(platform_now())
    }

    /// Time from `earlier` to `self`; zero when `earlier` is later.
    pub(crate) fn duration_since(self, earlier: Self) -> Duration {
        self.0.saturating_sub(earlier.0)
    }
}

/// A point on the test boot clock: Tokio's (pausable) clock plus the
/// simulated suspend time of this thread.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BootInstant {
    tokio: tokio::time::Instant,
    suspended: Duration,
}

#[cfg(test)]
thread_local! {
    static SUSPENDED: std::cell::Cell<Duration> = const { std::cell::Cell::new(Duration::ZERO) };
}

/// Simulate a host suspend of `by` on this thread's boot clock without
/// moving Tokio's clock (so no Tokio timer fires).
#[cfg(test)]
pub(crate) fn advance_boot_clock_for_test(by: Duration) {
    SUSPENDED.with(|suspended| suspended.set(suspended.get() + by));
}

#[cfg(test)]
impl BootInstant {
    pub(crate) fn now() -> Self {
        Self {
            tokio: tokio::time::Instant::now(),
            suspended: SUSPENDED.with(std::cell::Cell::get),
        }
    }

    /// Time from `earlier` to `self`; zero when `earlier` is later.
    pub(crate) fn duration_since(self, earlier: Self) -> Duration {
        self.tokio.saturating_duration_since(earlier.tokio)
            + self.suspended.saturating_sub(earlier.suspended)
    }
}

impl BootInstant {
    /// Time since `self` on the boot clock.
    pub(crate) fn elapsed(self) -> Duration {
        Self::now().duration_since(self)
    }
}

/// `CLOCK_BOOTTIME`: monotonic, and counts time the host spent suspended.
#[cfg(target_os = "linux")]
pub(crate) fn platform_now() -> Duration {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `now` is a valid, writable timespec for the duration of the call.
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut now) };
    assert_eq!(
        rc,
        0,
        "clock_gettime(CLOCK_BOOTTIME) failed: {}",
        std::io::Error::last_os_error()
    );
    Duration::new(now.tv_sec as u64, now.tv_nsec as u32)
}

/// Fallback for non-Linux development builds: a monotonic clock that may stop
/// while the host sleeps.
#[cfg(not(target_os = "linux"))]
pub(crate) fn platform_now() -> Duration {
    static BASE: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    BASE.get_or_init(std::time::Instant::now).elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_clock_is_monotone_and_not_behind_instant() {
        let started = std::time::Instant::now();
        let first = platform_now();
        std::thread::sleep(Duration::from_millis(20));
        let second = platform_now();
        let instant_elapsed = started.elapsed();
        assert!(second >= first);
        // The boot clock counts at least what the monotonic clock counts
        // (allowing for the reads not being simultaneous).
        assert!(second - first + Duration::from_millis(5) >= Duration::from_millis(20));
        assert!(second - first <= instant_elapsed + Duration::from_millis(5));
    }

    #[tokio::test(start_paused = true)]
    async fn test_clock_follows_tokio_and_simulated_suspend_without_firing_timers() {
        let started = BootInstant::now();
        tokio::time::advance(Duration::from_secs(5)).await;
        assert_eq!(started.elapsed(), Duration::from_secs(5));

        let tokio_before = tokio::time::Instant::now();
        advance_boot_clock_for_test(Duration::from_secs(60));
        assert_eq!(started.elapsed(), Duration::from_secs(65));
        assert_eq!(tokio::time::Instant::now(), tokio_before);
        assert_eq!(started.duration_since(BootInstant::now()), Duration::ZERO);
    }
}
