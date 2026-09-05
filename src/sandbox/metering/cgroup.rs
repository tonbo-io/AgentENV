//! cgroup v2 placement for Firecracker processes.
//!
//! The server owns the cgroup its container (or service) started it in. It
//! moves itself and every process already there into `agentenv/`, so the
//! parent no longer holds processes and may enable controllers for its
//! children, then keeps one leaf per sandbox under `sandboxes/`:
//!
//! ```text
//! <own cgroup>/
//! ├── cgroup.subtree_control   +cpu +memory
//! ├── agentenv/                the server, the ublk daemon, warm Firecrackers
//! └── sandboxes/
//!     ├── cgroup.subtree_control   +cpu +memory
//!     └── <sandbox id>/            one Firecracker process
//! ```
//!
//! The admission owner writes limits to these leaves when enabled. A warm-pool Firecracker
//! starts in `agentenv/` and moves into its sandbox leaf when claimed, so the
//! leaf charges the process from the moment it belongs to a sandbox.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use nix::unistd::Pid;
use tracing::{debug, warn};

use crate::types::SandboxId;

const PROC_SELF_CGROUP: &str = "/proc/self/cgroup";
const SERVER_LEAF: &str = "agentenv";
const SANDBOXES_DIR: &str = "sandboxes";
const CONTROLLERS: &str = "+cpu +memory";
/// Passes over `cgroup.procs` when emptying the parent. Processes forked
/// between a read and its writes need another pass; a pathological fork storm
/// would need more than this, and the controller write then reports it.
const MOVE_PASSES: usize = 3;

#[derive(Debug)]
pub(crate) struct CgroupTree {
    pub(crate) sandboxes_dir: PathBuf,
}

impl CgroupTree {
    /// Prepares the tree under the cgroup this process currently belongs to.
    pub fn init(cgroup_root: &Path) -> Result<Self> {
        let content = fs::read_to_string(PROC_SELF_CGROUP)
            .with_context(|| format!("read {PROC_SELF_CGROUP}"))?;
        let relative = parse_unified_cgroup_path(&content)
            .context("no cgroup v2 unified hierarchy entry in /proc/self/cgroup")?;
        Self::init_at(&join_cgroup_path(cgroup_root, relative))
    }

    /// Prepares the tree under an explicit cgroup directory.
    pub(crate) fn init_at(own: &Path) -> Result<Self> {
        let available = fs::read_to_string(own.join("cgroup.controllers"))
            .with_context(|| format!("{} is not a cgroup v2 directory", own.display()))?;
        for controller in ["cpu", "memory"] {
            if !available.split_whitespace().any(|name| name == controller) {
                bail!(
                    "the {controller} controller is not available in {}",
                    own.display()
                );
            }
        }

        let server_leaf = own.join(SERVER_LEAF);
        fs::create_dir_all(&server_leaf)
            .with_context(|| format!("create {}", server_leaf.display()))?;
        move_all_procs(own, &server_leaf)?;
        enable_controllers(own)?;
        // Kubernetes sets memory.oom.group=1 on the container cgroup, which
        // would take the server and every sandbox down when one Firecracker
        // is OOM-killed. A hook that clears it may run before or after this
        // takeover (runc puts later execs into the server's leaf), so the
        // owner of the tree clears it here regardless of ordering.
        if let Err(err) = fs::write(own.join("memory.oom.group"), "0") {
            debug!(cgroup = %own.display(), error = %err, "memory.oom.group left unchanged");
        }

        let sandboxes_dir = own.join(SANDBOXES_DIR);
        fs::create_dir_all(&sandboxes_dir)
            .with_context(|| format!("create {}", sandboxes_dir.display()))?;
        enable_controllers(&sandboxes_dir)?;
        sweep_stale_leaves(&sandboxes_dir);

        debug!(cgroup = %own.display(), "sandbox cgroup tree prepared");
        Ok(Self { sandboxes_dir })
    }

    /// Creates the sandbox's leaf and moves the Firecracker process into it.
    pub fn place(&self, sandbox_id: SandboxId, pid: Pid) -> Result<PathBuf> {
        let leaf = self.sandboxes_dir.join(sandbox_id.to_string());
        fs::create_dir_all(&leaf).with_context(|| format!("create {}", leaf.display()))?;
        write_pid(&leaf, pid).with_context(|| format!("move pid {pid} into {}", leaf.display()))?;
        Ok(leaf)
    }
}

/// Removes an empty sandbox leaf. Fails with `EBUSY` while a process is
/// still inside, which the caller retries once the process has exited.
pub(crate) fn remove_leaf(leaf: &Path) -> io::Result<()> {
    match fs::remove_dir(leaf) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// `usage_usec` from the leaf's `cpu.stat`.
pub(crate) fn read_cpu_usage_micros(leaf: &Path) -> Option<u64> {
    fs::read_to_string(leaf.join("cpu.stat"))
        .ok()
        .as_deref()
        .and_then(parse_cpu_stat_usage_usec)
}

/// The leaf's `memory.current`.
pub(crate) fn read_memory_current_bytes(leaf: &Path) -> Option<u64> {
    fs::read_to_string(leaf.join("memory.current"))
        .ok()
        .and_then(|content| content.trim().parse().ok())
}

/// The path of the unified (v2) hierarchy entry, `0::<path>`.
pub(crate) fn parse_unified_cgroup_path(proc_self_cgroup: &str) -> Option<&str> {
    proc_self_cgroup.lines().find_map(|line| {
        let mut fields = line.splitn(3, ':');
        let hierarchy = fields.next()?;
        let controllers = fields.next()?;
        let path = fields.next()?;
        (hierarchy == "0" && controllers.is_empty()).then_some(path)
    })
}

pub(crate) fn parse_cpu_stat_usage_usec(content: &str) -> Option<u64> {
    content.lines().find_map(|line| {
        let (key, value) = line.split_once(' ')?;
        (key == "usage_usec").then(|| value.trim().parse().ok())?
    })
}

fn join_cgroup_path(root: &Path, relative: &str) -> PathBuf {
    let relative = relative.trim_start_matches('/');
    if relative.is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

fn enable_controllers(dir: &Path) -> Result<()> {
    fs::write(dir.join("cgroup.subtree_control"), CONTROLLERS)
        .with_context(|| format!("enable {CONTROLLERS} in {}", dir.display()))
}

fn write_pid(leaf: &Path, pid: Pid) -> io::Result<()> {
    fs::write(leaf.join("cgroup.procs"), pid.to_string())
}

fn read_procs(dir: &Path) -> Result<Vec<Pid>> {
    let content = fs::read_to_string(dir.join("cgroup.procs"))
        .with_context(|| format!("read {}/cgroup.procs", dir.display()))?;
    Ok(content
        .split_whitespace()
        .filter_map(|raw| raw.parse::<i32>().ok())
        .map(Pid::from_raw)
        .collect())
}

/// Moves every process in `from` into `to` so `from` can enable controllers.
fn move_all_procs(from: &Path, to: &Path) -> Result<()> {
    for _ in 0..MOVE_PASSES {
        let pids = read_procs(from)?;
        if pids.is_empty() {
            return Ok(());
        }
        for pid in pids {
            match write_pid(to, pid) {
                Ok(()) => {}
                // The process exited between the read and the write.
                Err(err) if err.raw_os_error() == Some(libc::ESRCH) => {}
                Err(err) => {
                    return Err(err)
                        .with_context(|| format!("move pid {pid} into {}", to.display()));
                }
            }
        }
    }
    Ok(())
}

/// Removes leaves left behind by a previous server process. A leaf that still
/// holds a process belongs to an orphaned Firecracker and stays; it is not
/// this server's to kill.
fn sweep_stale_leaves(sandboxes_dir: &Path) {
    let Ok(entries) = fs::read_dir(sandboxes_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match remove_leaf(&path) {
            Ok(()) => debug!(leaf = %path.display(), "removed stale sandbox cgroup"),
            Err(err) => warn!(leaf = %path.display(), error = %err, "stale sandbox cgroup kept"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// A fake cgroup v2 directory: plain files stand in for the kernel's.
    fn fake_cgroup(dir: &Path, procs: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("cgroup.controllers"),
            "cpuset cpu io memory pids\n",
        )
        .unwrap();
        fs::write(dir.join("cgroup.procs"), procs).unwrap();
        fs::write(dir.join("cgroup.subtree_control"), "").unwrap();
    }

    #[test]
    fn unified_hierarchy_path_is_the_zero_entry() {
        let content = "12:memory:/legacy\n0::/kubepods.slice/pod-a/container-a\n";
        assert_eq!(
            parse_unified_cgroup_path(content),
            Some("/kubepods.slice/pod-a/container-a")
        );
        assert_eq!(parse_unified_cgroup_path("1:cpu:/only-v1\n"), None);
    }

    #[test]
    fn cpu_stat_usage_is_the_usage_usec_line() {
        let content = "usage_usec 123456\nuser_usec 100000\nsystem_usec 23456\n";
        assert_eq!(parse_cpu_stat_usage_usec(content), Some(123_456));
        assert_eq!(parse_cpu_stat_usage_usec("user_usec 1\n"), None);
    }

    #[test]
    fn join_cgroup_path_handles_the_root_cgroup() {
        let root = Path::new("/sys/fs/cgroup");
        assert_eq!(join_cgroup_path(root, "/"), root);
        assert_eq!(
            join_cgroup_path(root, "/a/b"),
            PathBuf::from("/sys/fs/cgroup/a/b")
        );
    }

    #[test]
    fn init_moves_processes_out_and_enables_controllers() -> Result<()> {
        let temp = tempdir()?;
        let own = temp.path().join("container");
        fake_cgroup(&own, "10\n11\n");
        let stale = own.join(SANDBOXES_DIR).join("stale-leaf");
        fs::create_dir_all(&stale)?;

        let tree = CgroupTree::init_at(&own)?;

        // Plain files do not consume writes the way cgroupfs does, so the
        // last pid written is what the fake leaf shows.
        assert_eq!(
            fs::read_to_string(own.join(SERVER_LEAF).join("cgroup.procs"))?,
            "11"
        );
        assert_eq!(
            fs::read_to_string(own.join("cgroup.subtree_control"))?,
            CONTROLLERS
        );
        assert_eq!(
            fs::read_to_string(tree.sandboxes_dir.join("cgroup.subtree_control"))?,
            CONTROLLERS
        );
        assert!(!stale.exists(), "stale empty leaf is removed at init");
        assert_eq!(fs::read_to_string(own.join("memory.oom.group"))?, "0");
        Ok(())
    }

    #[test]
    fn init_requires_cpu_and_memory_controllers() -> Result<()> {
        let temp = tempdir()?;
        let own = temp.path().join("container");
        fake_cgroup(&own, "");
        fs::write(own.join("cgroup.controllers"), "pids\n")?;

        let err = CgroupTree::init_at(&own).unwrap_err();
        assert!(err.to_string().contains("cpu controller"), "{err:#}");
        Ok(())
    }

    #[test]
    fn place_creates_a_leaf_named_after_the_sandbox() -> Result<()> {
        let temp = tempdir()?;
        let own = temp.path().join("container");
        fake_cgroup(&own, "");
        let tree = CgroupTree::init_at(&own)?;
        let sandbox_id = SandboxId::new();

        let leaf = tree.place(sandbox_id, Pid::from_raw(4242))?;

        assert_eq!(leaf, own.join(SANDBOXES_DIR).join(sandbox_id.to_string()));
        assert_eq!(fs::read_to_string(leaf.join("cgroup.procs"))?, "4242");
        fs::write(leaf.join("cpu.stat"), "usage_usec 77\nuser_usec 70\n")?;
        fs::write(leaf.join("memory.current"), "1048576\n")?;
        assert_eq!(read_cpu_usage_micros(&leaf), Some(77));
        assert_eq!(read_memory_current_bytes(&leaf), Some(1_048_576));

        fs::remove_file(leaf.join("cgroup.procs"))?;
        fs::remove_file(leaf.join("cpu.stat"))?;
        fs::remove_file(leaf.join("memory.current"))?;
        remove_leaf(&leaf)?;
        assert!(!leaf.exists());
        remove_leaf(&leaf)?;
        Ok(())
    }

    #[test]
    #[ignore = "requires root on a cgroup v2 host to write the server's own cgroup"]
    fn real_cgroup_accounts_a_child_process() -> Result<()> {
        // cgroupfs permissions are ownership-based, so CAP_SYS_ADMIN alone
        // (what the capability runner grants) cannot write them. The runner
        // deliberately avoids root, so this test is a host check rather than
        // a CI gate: it verifies where it can and reports where it cannot.
        if !nix::unistd::Uid::effective().is_root() {
            eprintln!("skipped: writing cgroupfs requires root");
            return Ok(());
        }
        let tree = CgroupTree::init(Path::new("/sys/fs/cgroup"))?;
        let mut child = std::process::Command::new("sh")
            .args([
                "-c",
                "i=0; while [ $i -lt 200000 ]; do i=$((i+1)); done; sleep 1",
            ])
            .spawn()?;
        let pid = Pid::from_raw(child.id() as i32);
        let leaf = tree.place(SandboxId::new(), pid)?;

        assert_eq!(
            fs::read_to_string(leaf.join("cgroup.procs"))?.trim(),
            pid.to_string()
        );
        child.wait()?;
        let cpu = read_cpu_usage_micros(&leaf).expect("cpu.stat readable");
        assert!(cpu > 0, "shell loop consumed CPU: {cpu}");
        assert!(read_memory_current_bytes(&leaf).is_some());
        remove_leaf(&leaf)?;
        Ok(())
    }
}
