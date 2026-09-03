use std::time::{Duration, SystemTime};

use uuid::Uuid;

/// One reading of a runtime instance's host resource use.
///
/// CPU and memory come from the instance's cgroup and are `None` when the
/// server could not place the Firecracker process in a cgroup of its own.
/// Disk is the number of bytes the runtime work directory currently occupies
/// on the node's local disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UsageSample {
    pub cpu_usage_micros: Option<u64>,
    pub memory_current_bytes: Option<u64>,
    pub disk_allocated_bytes: u64,
}

/// Cumulative host resource use of one runtime instance of a sandbox.
///
/// A runtime instance is one Firecracker process: it starts when the sandbox
/// boots or resumes on this node and ends when that process stops. Every
/// counter is monotonic within an instance and starts from zero for the next
/// one, so a consumer keyed by `runtime_instance_id` can tell a reset from a
/// decrease.
///
/// Byte-seconds integrate the gauge that was current at the start of each
/// sampling interval over that interval's length, so a gauge that changed
/// between two samples is charged at its earlier value until the later sample
/// observes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageCounters {
    pub runtime_instance_id: Uuid,
    pub started_at: SystemTime,
    pub sampled_at: SystemTime,
    pub sample_count: u64,
    /// `false` once the runtime instance stopped; the counters are then final.
    pub running: bool,
    /// Host CPU time consumed by every thread of the Firecracker process, in
    /// microseconds. Divide by one million for vCPU-seconds.
    pub cpu_usage_micros: Option<u64>,
    /// Bytes of host memory currently charged to the Firecracker process:
    /// anonymous guest memory it has touched plus page cache it owns.
    pub memory_current_bytes: Option<u64>,
    pub memory_byte_seconds: Option<u64>,
    pub disk_allocated_bytes: u64,
    pub disk_byte_seconds: u64,
}

impl UsageCounters {
    /// Opens the counters for a runtime instance from its first reading.
    pub fn start(now: SystemTime, first: UsageSample) -> Self {
        Self {
            runtime_instance_id: Uuid::now_v7(),
            started_at: now,
            sampled_at: now,
            sample_count: 1,
            running: true,
            cpu_usage_micros: first.cpu_usage_micros,
            memory_current_bytes: first.memory_current_bytes,
            memory_byte_seconds: first.memory_current_bytes.map(|_| 0),
            disk_allocated_bytes: first.disk_allocated_bytes,
            disk_byte_seconds: 0,
        }
    }

    /// Folds a new reading in, charging the previous gauges over `elapsed`.
    ///
    /// A cgroup reading that failed transiently keeps the previous value
    /// rather than dropping to `None`: a counter that has been available must
    /// not disappear from a running instance.
    pub fn advance(&mut self, elapsed: Duration, now: SystemTime, sample: UsageSample) {
        if let Some(previous) = self.memory_current_bytes {
            let charged = self.memory_byte_seconds.unwrap_or(0);
            self.memory_byte_seconds =
                Some(charged.saturating_add(byte_seconds(previous, elapsed)));
        }
        self.disk_byte_seconds = self
            .disk_byte_seconds
            .saturating_add(byte_seconds(self.disk_allocated_bytes, elapsed));

        // cgroup CPU time never decreases; a smaller reading means the
        // controller file was read before a previous write settled.
        self.cpu_usage_micros = match (self.cpu_usage_micros, sample.cpu_usage_micros) {
            (Some(previous), Some(current)) => Some(previous.max(current)),
            (previous, current) => current.or(previous),
        };
        self.memory_current_bytes = sample.memory_current_bytes.or(self.memory_current_bytes);
        if self.memory_current_bytes.is_some() && self.memory_byte_seconds.is_none() {
            self.memory_byte_seconds = Some(0);
        }
        self.disk_allocated_bytes = sample.disk_allocated_bytes;
        self.sampled_at = now;
        self.sample_count = self.sample_count.saturating_add(1);
    }

    /// Closes the counters after the runtime instance stopped.
    pub fn finish(&mut self, elapsed: Duration, now: SystemTime, last: UsageSample) {
        self.advance(elapsed, now, last);
        self.running = false;
    }
}

/// `bytes` held for `elapsed`, in byte-seconds, saturating at `u64::MAX`.
fn byte_seconds(bytes: u64, elapsed: Duration) -> u64 {
    let product = u128::from(bytes).saturating_mul(elapsed.as_micros());
    u64::try_from(product / 1_000_000).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn start_opens_with_first_reading_and_zero_integrals() {
        let counters = UsageCounters::start(
            at(100),
            UsageSample {
                cpu_usage_micros: Some(5_000),
                memory_current_bytes: Some(GIB),
                disk_allocated_bytes: 4096,
            },
        );

        assert!(counters.running);
        assert_eq!(counters.sample_count, 1);
        assert_eq!(counters.cpu_usage_micros, Some(5_000));
        assert_eq!(counters.memory_current_bytes, Some(GIB));
        assert_eq!(counters.memory_byte_seconds, Some(0));
        assert_eq!(counters.disk_allocated_bytes, 4096);
        assert_eq!(counters.disk_byte_seconds, 0);
    }

    #[test]
    fn advance_charges_the_previous_gauge_over_the_interval() {
        let mut counters = UsageCounters::start(
            at(0),
            UsageSample {
                cpu_usage_micros: Some(0),
                memory_current_bytes: Some(2 * GIB),
                disk_allocated_bytes: 1_000,
            },
        );

        counters.advance(
            Duration::from_secs(10),
            at(10),
            UsageSample {
                cpu_usage_micros: Some(7_500_000),
                memory_current_bytes: Some(4 * GIB),
                disk_allocated_bytes: 3_000,
            },
        );

        assert_eq!(counters.memory_byte_seconds, Some(20 * GIB));
        assert_eq!(counters.disk_byte_seconds, 10_000);
        assert_eq!(counters.memory_current_bytes, Some(4 * GIB));
        assert_eq!(counters.disk_allocated_bytes, 3_000);
        assert_eq!(counters.cpu_usage_micros, Some(7_500_000));
        assert_eq!(counters.sample_count, 2);
        assert_eq!(counters.sampled_at, at(10));

        counters.advance(
            Duration::from_millis(2_500),
            at(13),
            UsageSample {
                cpu_usage_micros: Some(8_000_000),
                memory_current_bytes: Some(GIB),
                disk_allocated_bytes: 3_000,
            },
        );
        assert_eq!(counters.memory_byte_seconds, Some(30 * GIB));
        assert_eq!(counters.disk_byte_seconds, 17_500);
    }

    #[test]
    fn transient_cgroup_read_failure_keeps_previous_values() {
        let mut counters = UsageCounters::start(
            at(0),
            UsageSample {
                cpu_usage_micros: Some(100),
                memory_current_bytes: Some(GIB),
                disk_allocated_bytes: 0,
            },
        );

        counters.advance(Duration::from_secs(1), at(1), UsageSample::default());

        assert_eq!(counters.cpu_usage_micros, Some(100));
        assert_eq!(counters.memory_current_bytes, Some(GIB));
        assert_eq!(counters.memory_byte_seconds, Some(GIB));
    }

    #[test]
    fn cpu_counter_never_decreases() {
        let mut counters = UsageCounters::start(
            at(0),
            UsageSample {
                cpu_usage_micros: Some(900),
                ..UsageSample::default()
            },
        );

        counters.advance(
            Duration::from_secs(1),
            at(1),
            UsageSample {
                cpu_usage_micros: Some(800),
                ..UsageSample::default()
            },
        );

        assert_eq!(counters.cpu_usage_micros, Some(900));
    }

    #[test]
    fn cgroup_counters_stay_absent_without_a_cgroup() {
        let mut counters = UsageCounters::start(
            at(0),
            UsageSample {
                disk_allocated_bytes: 512,
                ..UsageSample::default()
            },
        );
        counters.advance(
            Duration::from_secs(4),
            at(4),
            UsageSample {
                disk_allocated_bytes: 512,
                ..UsageSample::default()
            },
        );

        assert_eq!(counters.cpu_usage_micros, None);
        assert_eq!(counters.memory_current_bytes, None);
        assert_eq!(counters.memory_byte_seconds, None);
        assert_eq!(counters.disk_byte_seconds, 2_048);
    }

    #[test]
    fn finish_charges_the_last_interval_and_stops() {
        let mut counters = UsageCounters::start(
            at(0),
            UsageSample {
                memory_current_bytes: Some(GIB),
                disk_allocated_bytes: 100,
                ..UsageSample::default()
            },
        );

        counters.finish(
            Duration::from_secs(3),
            at(3),
            UsageSample {
                memory_current_bytes: Some(0),
                disk_allocated_bytes: 0,
                ..UsageSample::default()
            },
        );

        assert!(!counters.running);
        assert_eq!(counters.memory_byte_seconds, Some(3 * GIB));
        assert_eq!(counters.disk_byte_seconds, 300);
        assert_eq!(counters.memory_current_bytes, Some(0));
    }

    #[test]
    fn byte_seconds_saturates_instead_of_overflowing() {
        assert_eq!(
            byte_seconds(u64::MAX, Duration::from_secs(u64::MAX)),
            u64::MAX
        );
        assert_eq!(byte_seconds(1_000_000, Duration::from_micros(1)), 1);
    }
}
