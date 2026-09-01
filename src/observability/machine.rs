use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::MachineInfo;

/// Inputs required to dump the guest-visible CPU configuration used as a
/// cross-node snapshot compatibility fact.
#[derive(Clone, Debug)]
pub struct CpuTemplateDumpConfig {
    helper_path: PathBuf,
    kernel_image_path: PathBuf,
}

impl CpuTemplateDumpConfig {
    pub fn new(helper_path: PathBuf, kernel_image_path: PathBuf) -> Self {
        Self {
            helper_path,
            kernel_image_path,
        }
    }
}

/// Detects mostly-static machine descriptors for inclusion in node snapshots.
///
/// This is evaluated once during observability service construction rather than
/// on every API request.
pub(crate) fn detect_machine_info() -> MachineInfo {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    MachineInfo {
        cpu_family: first_cpuinfo_value(&cpuinfo, &["cpu family", "CPU architecture"])
            .unwrap_or_else(|| "unknown".to_string()),
        cpu_model: first_cpuinfo_value(&cpuinfo, &["model", "CPU part"])
            .unwrap_or_else(|| "unknown".to_string()),
        cpu_model_name: first_cpuinfo_value(&cpuinfo, &["model name", "Hardware"])
            .unwrap_or_else(|| "unknown".to_string()),
        cpu_architecture: std::env::consts::ARCH.to_string(),
        cpu_config_json: None,
    }
}

/// Runs `cpu-template-helper template dump` against the configured Firecracker
/// kernel and returns canonical JSON. An explicit Firecracker config is
/// required on aarch64, where the helper's built-in boot source is not a valid
/// kernel for the host architecture.
pub(super) async fn dump_cpu_config(config: CpuTemplateDumpConfig) -> Result<String> {
    tokio::task::spawn_blocking(move || dump_cpu_config_blocking(config))
        .await
        .context("cpu-template-helper task failed")?
}

fn dump_cpu_config_blocking(config: CpuTemplateDumpConfig) -> Result<String> {
    let mut firecracker_config =
        tempfile::NamedTempFile::new().context("create cpu-template-helper config")?;
    serde_json::to_writer(
        firecracker_config.as_file_mut(),
        &serde_json::json!({
            "boot-source": {
                "kernel_image_path": config.kernel_image_path,
                "boot_args": ""
            },
            "drives": [],
            "network-interfaces": []
        }),
    )
    .context("serialize cpu-template-helper config")?;
    firecracker_config
        .as_file_mut()
        .flush()
        .context("flush cpu-template-helper config")?;

    let output = std::process::Command::new(&config.helper_path)
        .args(["template", "dump", "--config"])
        .arg(firecracker_config.path())
        .args(["--output", "/dev/stdout"])
        .output()
        .with_context(|| {
            format!(
                "run cpu-template-helper at {}",
                config.helper_path.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "cpu-template-helper exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let raw = String::from_utf8(output.stdout).context("cpu-template-helper returned non-UTF-8")?;
    if raw.trim().is_empty() {
        bail!("cpu-template-helper returned an empty CPU configuration");
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).context("cpu-template-helper returned invalid JSON")?;
    serde_json::to_string(&parsed).context("canonicalize cpu-template-helper output")
}

pub(crate) fn first_cpuinfo_value(cpuinfo: &str, keys: &[&str]) -> Option<String> {
    cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        keys.iter()
            .any(|candidate| key.trim() == *candidate)
            .then(|| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{dump_cpu_config, first_cpuinfo_value, CpuTemplateDumpConfig};

    #[test]
    fn cpuinfo_parser_returns_first_matching_key() {
        let cpuinfo = "model name\t: Intel(R)\ncpu family\t: 6\n";
        assert_eq!(
            first_cpuinfo_value(cpuinfo, &["cpu family"]).as_deref(),
            Some("6")
        );
        assert_eq!(
            first_cpuinfo_value(cpuinfo, &["model name"]).as_deref(),
            Some("Intel(R)")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cpu_template_dump_uses_explicit_kernel_config() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("cpu-template-helper");
        std::fs::write(
            &helper,
            r#"#!/bin/sh
set -eu
config=
output=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --config) config="$2"; shift 2 ;;
    --output) output="$2"; shift 2 ;;
    *) shift ;;
  esac
done
test "$output" = /dev/stdout
grep -Fq '"kernel_image_path":"/kernel/vmlinux.bin"' "$config"
printf '%s\n' '{ "reg_modifiers": [], "kvm_capabilities": [] }'
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&helper, permissions).unwrap();

        let dumped = dump_cpu_config(CpuTemplateDumpConfig::new(
            helper,
            PathBuf::from("/kernel/vmlinux.bin"),
        ))
        .await
        .unwrap();

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&dumped).unwrap(),
            serde_json::json!({"kvm_capabilities": [], "reg_modifiers": []})
        );
    }
}
