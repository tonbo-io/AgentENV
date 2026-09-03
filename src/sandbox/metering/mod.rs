//! Per-sandbox host resource metering.
//!
//! Each Firecracker process gets a cgroup v2 leaf of its own (see
//! [`cgroup`]). A background sampler reads the leaf's CPU time and resident
//! memory together with the allocated size of the runtime work directory,
//! and folds them into monotonic [`UsageCounters`] per runtime instance.
//! Counters answer "what did this sandbox consume on this node", which the
//! control plane needs when sandboxes are provisioned larger than they use
//! and the node is oversubscribed.
//!
//! The meter is a side registry keyed by [`SandboxId`]: sampling never takes
//! a sandbox's backend lock, so a long pause or snapshot cannot stall it.

mod cgroup;
mod counters;
mod disk;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use nix::unistd::Pid;
use tracing::{debug, info, warn};

use crate::cfg::MeteringConfig;
use crate::types::SandboxId;

pub use counters::{UsageCounters, UsageSample};

static METER: OnceLock<Option<&'static SandboxMeter>> = OnceLock::new();

/// Process-wide registry of runtime instance usage counters.
pub struct SandboxMeter {
    cgroups: Option<cgroup::CgroupTree>,
    entries: Mutex<HashMap<SandboxId, MeterEntry>>,
    finished_retention: Duration,
}

struct MeterEntry {
    /// Where the next sample is read from; `None` once the instance stopped.
    source: Option<UsageSource>,
    /// A leaf whose removal failed because its process had not exited yet.
    pending_leaf: Option<PathBuf>,
    counters: UsageCounters,
    last_sample: Instant,
    finished_at: Option<Instant>,
}

#[derive(Clone, Debug)]
struct UsageSource {
    cgroup_leaf: Option<PathBuf>,
    work_dir: PathBuf,
}

impl UsageSource {
    fn read(&self) -> UsageSample {
        UsageSample {
            cpu_usage_micros: self
                .cgroup_leaf
                .as_deref()
                .and_then(cgroup::read_cpu_usage_micros),
            memory_current_bytes: self
                .cgroup_leaf
                .as_deref()
                .and_then(cgroup::read_memory_current_bytes),
            disk_allocated_bytes: disk::allocated_bytes(&self.work_dir),
        }
    }
}

impl SandboxMeter {
    /// Installs the process-wide meter and starts its sampler.
    ///
    /// Without a writable cgroup v2 tree the meter still runs and reports
    /// disk usage; CPU and memory counters are then absent, and the node log
    /// says so once at startup.
    pub fn init_global(config: &MeteringConfig) -> Result<()> {
        if METER.get().is_some() {
            return Ok(());
        }
        if !config.enabled {
            let _ = METER.set(None);
            info!("sandbox metering disabled by configuration");
            return Ok(());
        }
        let cgroups = match cgroup::CgroupTree::init(&config.cgroup_root) {
            Ok(tree) => Some(tree),
            Err(err) => {
                warn!(
                    error = %format!("{err:#}"),
                    cgroup_root = %config.cgroup_root.display(),
                    "sandbox cgroup accounting unavailable; metering CPU and memory as absent"
                );
                None
            }
        };
        let meter: &'static SandboxMeter = Box::leak(Box::new(Self {
            cgroups,
            entries: Mutex::new(HashMap::new()),
            finished_retention: Duration::from_secs(config.finished_retention_secs),
        }));
        if METER.set(Some(meter)).is_err() {
            return Ok(());
        }
        let interval = Duration::from_secs(config.sample_interval_secs.max(1));
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if let Err(err) = tokio::task::spawn_blocking(move || meter.sample_all()).await {
                    warn!(error = %err, "sandbox metering sample pass panicked");
                }
            }
        });
        info!(
            interval_secs = interval.as_secs(),
            cgroup_accounting = meter.cgroups.is_some(),
            "sandbox metering started"
        );
        Ok(())
    }

    /// The installed meter, or `None` when metering is disabled or the
    /// process never installed one (library use, tests).
    pub fn global() -> Option<&'static Self> {
        METER.get().copied().flatten()
    }

    /// Opens counters for a runtime instance whose Firecracker process is
    /// `pid` and whose runtime files live under `work_dir`.
    ///
    /// A previous instance of the same sandbox is replaced; its final
    /// counters are gone, so callers read them before resuming.
    pub fn attach(&self, sandbox_id: SandboxId, pid: Option<Pid>, work_dir: &Path) {
        let cgroup_leaf = match (self.cgroups.as_ref(), pid) {
            (Some(tree), Some(pid)) => match tree.place(sandbox_id, pid) {
                Ok(leaf) => Some(leaf),
                Err(err) => {
                    warn!(
                        sandbox = %sandbox_id,
                        error = %format!("{err:#}"),
                        "sandbox cgroup placement failed; CPU and memory counters absent"
                    );
                    None
                }
            },
            _ => None,
        };
        let source = UsageSource {
            cgroup_leaf,
            work_dir: work_dir.to_path_buf(),
        };
        let counters = UsageCounters::start(SystemTime::now(), source.read());
        let now = Instant::now();
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(previous) = entries.insert(
            sandbox_id,
            MeterEntry {
                source: Some(source),
                pending_leaf: None,
                counters,
                last_sample: now,
                finished_at: None,
            },
        ) {
            release_leaf(previous);
        }
        debug!(sandbox = %sandbox_id, "sandbox runtime metering attached");
    }

    /// Takes the final reading of a runtime instance and releases its cgroup.
    ///
    /// The final counters stay readable for the configured retention so the
    /// control plane can settle an instance it did not stop itself.
    pub fn detach(&self, sandbox_id: SandboxId) {
        let now = Instant::now();
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(entry) = entries.get_mut(&sandbox_id) else {
            return;
        };
        let Some(source) = entry.source.take() else {
            return;
        };
        let last = source.read();
        entry.counters.finish(
            now.duration_since(entry.last_sample),
            SystemTime::now(),
            last,
        );
        entry.last_sample = now;
        entry.finished_at = Some(now);
        if let Some(leaf) = source.cgroup_leaf {
            if let Err(err) = cgroup::remove_leaf(&leaf) {
                debug!(sandbox = %sandbox_id, error = %err, "sandbox cgroup removal deferred");
                entry.pending_leaf = Some(leaf);
            }
        }
        debug!(sandbox = %sandbox_id, "sandbox runtime metering detached");
    }

    /// The latest counters for a sandbox's most recent runtime instance on
    /// this node.
    pub fn usage(&self, sandbox_id: &SandboxId) -> Option<UsageCounters> {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        entries.get(sandbox_id).map(|entry| entry.counters.clone())
    }

    /// One sampling pass over every running instance. Also retries deferred
    /// cgroup removals and forgets finished instances past retention.
    fn sample_all(&self) {
        let sources: Vec<(SandboxId, UsageSource)> = {
            let entries = self
                .entries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            entries
                .iter()
                .filter_map(|(id, entry)| entry.source.clone().map(|source| (*id, source)))
                .collect()
        };
        let samples: Vec<(SandboxId, UsageSample)> = sources
            .into_iter()
            .map(|(id, source)| (id, source.read()))
            .collect();

        let now = Instant::now();
        let wall = SystemTime::now();
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for (id, sample) in samples {
            // An instance detached while its sample was being read has final
            // counters already; the stale reading is dropped.
            if let Some(entry) = entries.get_mut(&id).filter(|entry| entry.source.is_some()) {
                entry
                    .counters
                    .advance(now.duration_since(entry.last_sample), wall, sample);
                entry.last_sample = now;
            }
        }
        entries.retain(|id, entry| {
            if let Some(leaf) = entry.pending_leaf.take() {
                if let Err(err) = cgroup::remove_leaf(&leaf) {
                    debug!(sandbox = %id, error = %err, "sandbox cgroup removal still deferred");
                    entry.pending_leaf = Some(leaf);
                }
            }
            match entry.finished_at {
                Some(finished) if now.duration_since(finished) >= self.finished_retention => {
                    if let Some(leaf) = entry.pending_leaf.take() {
                        warn!(sandbox = %id, leaf = %leaf.display(), "sandbox cgroup leaf abandoned after retention");
                    }
                    false
                }
                _ => true,
            }
        });
    }
}

fn release_leaf(previous: MeterEntry) {
    let leaf = previous
        .source
        .and_then(|source| source.cgroup_leaf)
        .or(previous.pending_leaf);
    if let Some(leaf) = leaf {
        if let Err(err) = cgroup::remove_leaf(&leaf) {
            warn!(leaf = %leaf.display(), error = %err, "replaced sandbox cgroup leaf not removed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn meter() -> SandboxMeter {
        SandboxMeter {
            cgroups: None,
            entries: Mutex::new(HashMap::new()),
            finished_retention: Duration::from_secs(60),
        }
    }

    #[test]
    fn attach_sample_detach_produces_final_disk_counters() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("upper.data"), vec![1u8; 8192]).unwrap();
        let meter = meter();
        let id = SandboxId::new();

        meter.attach(id, None, temp.path());
        let opened = meter.usage(&id).unwrap();
        assert!(opened.running);
        assert!(opened.disk_allocated_bytes >= 8192);
        assert_eq!(opened.cpu_usage_micros, None);

        meter.sample_all();
        let sampled = meter.usage(&id).unwrap();
        assert_eq!(sampled.sample_count, 2);
        assert_eq!(sampled.runtime_instance_id, opened.runtime_instance_id);

        meter.detach(id);
        let finished = meter.usage(&id).unwrap();
        assert!(!finished.running);
        assert_eq!(finished.sample_count, 3);

        meter.detach(id);
        assert_eq!(meter.usage(&id).unwrap().sample_count, 3);
        meter.sample_all();
        assert_eq!(meter.usage(&id).unwrap().sample_count, 3);
    }

    #[test]
    fn reattach_starts_a_new_runtime_instance() {
        let temp = tempdir().unwrap();
        let meter = meter();
        let id = SandboxId::new();

        meter.attach(id, None, temp.path());
        let first = meter.usage(&id).unwrap();
        meter.detach(id);
        meter.attach(id, None, temp.path());
        let second = meter.usage(&id).unwrap();

        assert_ne!(first.runtime_instance_id, second.runtime_instance_id);
        assert!(second.running);
        assert_eq!(second.sample_count, 1);
    }

    #[test]
    fn finished_instances_are_forgotten_after_retention() {
        let temp = tempdir().unwrap();
        let meter = SandboxMeter {
            finished_retention: Duration::ZERO,
            ..meter()
        };
        let id = SandboxId::new();

        meter.attach(id, None, temp.path());
        meter.detach(id);
        assert!(meter.usage(&id).is_some());
        meter.sample_all();
        assert!(meter.usage(&id).is_none());
    }

    #[test]
    fn unknown_sandboxes_have_no_usage() {
        assert!(meter().usage(&SandboxId::new()).is_none());
    }
}
