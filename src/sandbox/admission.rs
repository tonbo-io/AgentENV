//! The target node owns admission, resident limits and execution watchdogs.
//! Every Firecracker start goes through this registry before guest execution.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use runtime_policy::watchdog::{ProcessHandle, Watchdog};
use runtime_policy::{Budget, ExecutionLease, Reservation};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::cfg::AdmissionConfig;
use crate::local_store::{LocalKvStore, LocalStoreDurability};
use crate::sandbox::SandboxMeter;
use crate::types::{SandboxId, SandboxResources};

static ADMISSION: OnceLock<Option<Arc<NodeAdmission>>> = OnceLock::new();

pub struct NodeAdmission {
    config: AdmissionConfig,
    root: PathBuf,
    reserve_memory: u64,
    ledger: Mutex<Ledger>,
    identities: LocalKvStore,
    // Serializes durable activation claims with admission and kernel writes.
    claims: tokio::sync::Mutex<()>,
}

struct Ledger {
    budget: Budget,
    entries: HashMap<Uuid, Entry>,
}

struct Entry {
    sandbox_id: SandboxId,
    leaf: PathBuf,
    disk_path: PathBuf,
    process: Option<ProcessHandle>,
    watchdog: Option<Watchdog>,
    released: bool,
}

pub struct AdmissionGuard {
    owner: Arc<NodeAdmission>,
    id: Uuid,
}

impl NodeAdmission {
    pub async fn init_global(config: &AdmissionConfig) -> Result<()> {
        if !config.enabled {
            if config.require_execution_lease {
                bail!("required funding needs node admission enabled");
            }
            ADMISSION
                .set(None)
                .map_err(|_| anyhow::anyhow!("admission already initialized"))?;
            return Ok(());
        }
        if !(1..100).contains(&config.memory_percent)
            || config.initial_memory_bytes == 0
            || config.maximum_funded_seconds == 0
        {
            bail!(
                "admission must leave node memory headroom and allocate a positive initial budget"
            );
        }
        let root = SandboxMeter::global()
            .context("admission requires metering")?
            .cgroup_directory()?;
        let (capacity, _) = memory_capacity(&root)?;
        let budget = capacity / 100 * config.memory_percent;
        let identities = LocalKvStore::open(&config.state_path, LocalStoreDurability::Sync).await?;
        let owner = Arc::new(Self {
            config: config.clone(),
            root,
            reserve_memory: capacity - budget,
            ledger: Mutex::new(Ledger {
                budget: Budget::new(budget, config.max_starting)?,
                entries: HashMap::new(),
            }),
            identities,
            claims: tokio::sync::Mutex::new(()),
        });
        ADMISSION
            .set(Some(Arc::clone(&owner)))
            .map_err(|_| anyhow::anyhow!("admission already initialized"))?;
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_millis(100));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                timer.tick().await;
                let owner = Arc::clone(&owner);
                let _ = tokio::task::spawn_blocking(move || owner.reconcile()).await;
            }
        });
        info!(memory_budget_bytes = budget, "node admission enabled");
        Ok(())
    }

    pub fn global() -> Option<&'static Arc<Self>> {
        ADMISSION.get().and_then(Option::as_ref)
    }

    pub async fn acquire(
        self: &Arc<Self>,
        sandbox: SandboxId,
        resources: SandboxResources,
        lease: Option<ExecutionLease>,
        disk_path: &Path,
    ) -> Result<AdmissionGuard> {
        let _claim = self.claims.lock().await;
        self.reconcile();
        if self.config.require_execution_lease && lease.is_none() {
            bail!("a funded execution lease is required");
        }
        let id = lease.map_or_else(Uuid::now_v7, |lease| lease.activation_id);
        if let Some(lease) = lease {
            if lease.sequence != 0 {
                bail!("new activation must begin with execution sequence zero");
            }
            self.validate_lease(lease)?;
            if self.identities.get(id.as_bytes().to_vec()).await?.is_some() {
                bail!("activation has already executed on this node");
            }
        }
        let maximum_memory_bytes =
            u64::from(resources.memory_mib) * 1024 * 1024 + self.config.runtime_overhead_bytes;
        let reservation = Reservation {
            memory_bytes: maximum_memory_bytes.min(self.config.initial_memory_bytes),
            maximum_memory_bytes,
            // Also reserve space for one uncompressed memory checkpoint.
            disk_bytes: u64::from(resources.disk_size_mib) * 1024 * 1024
                + maximum_memory_bytes.min(self.config.initial_memory_bytes),
            starting: true,
        };
        let (_, available) = memory_capacity(&self.root)?;
        let disk_available =
            disk_available(disk_path)?.saturating_sub(self.config.disk_reserve_bytes);
        let leaf = self.root.join(sandbox.to_string());
        {
            let mut ledger = self.ledger.lock().unwrap();
            if ledger
                .entries
                .values()
                .any(|entry| entry.sandbox_id == sandbox)
            {
                bail!("sandbox already owns a runtime admission");
            }
            ledger.budget.admit(
                id,
                reservation,
                available.saturating_sub(self.reserve_memory),
                disk_available,
            )?;
            let configured = configure_leaf(&leaf, reservation.memory_bytes, resources.cpu_count);
            if let Err(error) = configured {
                ledger.budget.reservations.remove(&id);
                return Err(error);
            }
            ledger.entries.insert(
                id,
                Entry {
                    sandbox_id: sandbox,
                    leaf,
                    disk_path: disk_path.to_path_buf(),
                    process: None,
                    watchdog: None,
                    released: false,
                },
            );
        }
        let guard = AdmissionGuard {
            owner: Arc::clone(self),
            id,
        };
        if lease.is_some() {
            // Never reuse a physical activation after a crash or a lost reply.
            // Expired initial authorizations cannot recreate a cleared runtime.
            self.identities
                .put(id.as_bytes().to_vec(), sandbox.to_string().into_bytes())
                .await?;
        }
        Ok(guard)
    }

    fn validate_lease(&self, lease: ExecutionLease) -> Result<()> {
        if lease.remaining(SystemTime::now())?
            > Duration::from_secs(self.config.maximum_funded_seconds)
        {
            bail!("funded execution exceeds the node's maximum authorization window");
        }
        Ok(())
    }

    pub async fn renew(self: &Arc<Self>, sandbox: SandboxId, lease: ExecutionLease) -> Result<()> {
        self.validate_lease(lease)?;
        let owner = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let mut ledger = owner.ledger.lock().unwrap();
            let entry = ledger
                .entries
                .get_mut(&lease.activation_id)
                .context("activation has no admission")?;
            if entry.sandbox_id != sandbox || entry.released {
                bail!("execution lease lost its runtime fence");
            }
            entry
                .watchdog
                .as_mut()
                .context("runtime has no execution watchdog")?
                .renew(lease)
        })
        .await?
    }

    pub fn runtime_dead(&self, sandbox: SandboxId) -> bool {
        self.ledger
            .lock()
            .unwrap()
            .entries
            .values()
            .find(|entry| entry.sandbox_id == sandbox)
            .is_some_and(|entry| entry.process.as_ref().is_some_and(ProcessHandle::exited))
    }

    fn reconcile(&self) {
        let Ok((_, available)) = memory_capacity(&self.root) else {
            return;
        };
        let mut available = available.saturating_sub(self.reserve_memory);
        let mut ledger = self.ledger.lock().unwrap();
        let ids = ledger.entries.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let entry = &ledger.entries[&id];
            if entry.released && entry.process.as_ref().is_none_or(ProcessHandle::exited) {
                if fs::remove_dir(&entry.leaf).is_ok() || !entry.leaf.exists() {
                    ledger.entries.remove(&id);
                    ledger.budget.reservations.remove(&id);
                }
                continue;
            }
            let Some(current) = read_number(&entry.leaf.join("memory.current")) else {
                continue;
            };
            let old = ledger.budget.reservations[&id].memory_bytes;
            if current < old / 2 {
                continue;
            }
            let Ok(free_disk) = disk_available(&entry.disk_path) else {
                continue;
            };
            if let Some(next) = ledger.budget.growth(
                id,
                old.saturating_mul(2),
                available,
                free_disk.saturating_sub(self.config.disk_reserve_bytes),
            ) {
                if let Err(error) = set_memory(&entry.leaf, next) {
                    // A partly applied growth remains reserved conservatively.
                    error!(%error, %id, "runtime memory growth failed");
                }
                let reservation = ledger.budget.reservations.get_mut(&id).unwrap();
                reservation.memory_bytes = next;
                reservation.disk_bytes = reservation.disk_bytes.saturating_add(next - old);
                available = available.saturating_sub(next - old);
            }
        }
    }
}

impl AdmissionGuard {
    pub async fn attach(&self, pid: i32, lease: Option<ExecutionLease>) -> Result<()> {
        let process = ProcessHandle::open(pid)?;
        let owner = Arc::clone(&self.owner);
        let id = self.id;
        tokio::task::spawn_blocking(move || {
            let mut ledger = owner.ledger.lock().unwrap();
            let entry = ledger
                .entries
                .get_mut(&id)
                .context("admission disappeared before spawn")?;
            entry.process = Some(process);
            fs::write(entry.leaf.join("cgroup.procs"), pid.to_string())?;
            if let Some(lease) = lease {
                owner.validate_lease(lease)?;
                entry.watchdog = Some(Watchdog::start(&std::env::current_exe()?, pid, lease)?);
            }
            Ok(())
        })
        .await?
    }

    pub fn ready(&self) {
        if let Some(reservation) = self
            .owner
            .ledger
            .lock()
            .unwrap()
            .budget
            .reservations
            .get_mut(&self.id)
        {
            reservation.starting = false;
        }
    }
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        if let Some(entry) = self.owner.ledger.lock().unwrap().entries.get_mut(&self.id) {
            entry.released = true;
            if let Some(process) = &entry.process {
                if let Err(error) = process.kill() {
                    warn!(%error, "runtime admission cleanup could not kill its process");
                }
            }
            entry.watchdog.take();
        }
        // Keep capacity reserved until the kernel confirms process exit and
        // the cgroup is empty. The reconciler then releases it exactly once.
    }
}

fn set_memory(leaf: &Path, bytes: u64) -> Result<()> {
    fs::write(leaf.join("memory.max"), bytes.to_string())?;
    fs::write(leaf.join("memory.high"), (bytes / 2).to_string())?;
    Ok(())
}

fn configure_leaf(leaf: &Path, memory: u64, cpu: u32) -> Result<()> {
    if cpu == 0 {
        bail!("runtime CPU ceiling must be positive");
    }
    fs::create_dir_all(leaf)?;
    set_memory(leaf, memory)?;
    fs::write(leaf.join("memory.swap.max"), "0")?;
    fs::write(leaf.join("memory.oom.group"), "1")?;
    fs::write(leaf.join("cpu.weight"), "100")?;
    fs::write(
        leaf.join("cpu.max"),
        format!("{} 100000", u64::from(cpu) * 100000),
    )?;
    Ok(())
}

fn read_number(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn memory_capacity(root: &Path) -> Result<(u64, u64)> {
    let meminfo = fs::read_to_string("/proc/meminfo")?;
    let read = |name: &str| -> Result<u64> {
        let value = meminfo
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|value| value.split_whitespace().next())
            .context("host memory report is incomplete")?;
        Ok(value.parse::<u64>()?.saturating_mul(1024))
    };
    let mut capacity = read("MemTotal:")?;
    let mut available = read("MemAvailable:")?;
    // Every finite ancestor matters, including a container limit below the
    // host capacity. Reading only /proc/meminfo would over-admit test Pods.
    for ancestor in root.parent().into_iter().flat_map(Path::ancestors) {
        if let Some(limit) = read_number(&ancestor.join("memory.max")) {
            let current = read_number(&ancestor.join("memory.current"))
                .context("cgroup memory report is incomplete")?;
            capacity = capacity.min(limit);
            available = available.min(limit.saturating_sub(current));
        }
    }
    Ok((capacity, available))
}

fn disk_available(path: &Path) -> Result<u64> {
    let stats = nix::sys::statvfs::statvfs(path)?;
    Ok(stats
        .blocks_available()
        .saturating_mul(stats.fragment_size()))
}
