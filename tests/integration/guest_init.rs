use crate::common;

use agentenv::sandbox::{ExtraDrive, FirecrackerSandbox, SandboxBackend, SandboxExecutor};
use anyhow::Result;
use tokio::time::Duration;

const TEST_TIMEOUT: Duration = Duration::from_secs(90);

async fn assert_guest_boot_fails_closed(
    mut sandbox_config: agentenv::sandbox::FirecrackerSandboxConfig,
    expected_serial_log: &str,
) -> Result<()> {
    sandbox_config.common.runtime_policy.envd_timeout = Duration::from_secs(5);
    let mut sandbox = FirecrackerSandbox::new(sandbox_config)?;
    let serial_log = sandbox.firecracker_stdout_path();
    let result = sandbox.start().await;
    assert!(result.is_err(), "guest unexpectedly started: {result:?}");
    let mut output = String::new();
    for _ in 0..50 {
        output = std::fs::read_to_string(&serial_log)?;
        if output.contains(expected_serial_log) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _ = sandbox.stop().await;
    assert!(
        output.contains(expected_serial_log),
        "serial log omitted {expected_serial_log:?}: {output}"
    );
    assert!(
        !output.contains("started envd"),
        "envd started after a failed bootstrap: {output}"
    );
    Ok(())
}

#[tokio::test]
async fn critical_guest_bootstrap_failures_prevent_envd_start() -> Result<()> {
    common::setup().await;
    tokio::time::timeout(TEST_TIMEOUT, async {
        for step in ["devpts", "shared-memory", "dns", "loopback"] {
            let mut sandbox_config = common::default_sandbox_config()?;
            let boot_args = sandbox_config
                .boot_args
                .take()
                .expect("default sandbox boot arguments");
            sandbox_config.boot_args =
                Some(format!("{boot_args} agentenv_bootstrap_failpoint={step}"));
            assert_guest_boot_fails_closed(
                sandbox_config,
                &format!("bootstrap failed: injected {step} failure"),
            )
            .await?;
        }
        Ok(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("test timed out after {:?}", TEST_TIMEOUT))?
}

#[tokio::test]
async fn guest_uses_platform_init_process_tree() -> Result<()> {
    common::setup().await;
    tokio::time::timeout(TEST_TIMEOUT, async {
        let sandbox_config = common::default_sandbox_config()?;
        let mut sandbox = FirecrackerSandbox::new(sandbox_config)?;
        sandbox.start().await?;

        let process_tree = sandbox
            .run_command(
                "sh",
                &[
                    "-c",
                    "[ /proc/1/exe -ef /agentenv/agentenv-init ] && echo 'pid1=agentenv-init'; printf 'pid1_comm='; cat /proc/1/comm; for proc in /proc/[0-9]*; do [ \"$(cat \"$proc/comm\" 2>/dev/null)\" = envd ] || continue; awk '/^Pid:|^PPid:/{printf \"%s=%s \", $1, $2} END{print \"comm=envd\"}' \"$proc/status\"; done",
                ],
            )
            .await?;

        assert!(
            process_tree.stdout.contains("pid1=agentenv-init"),
            "expected agentenv-init as PID 1, got: {:?}",
            process_tree.stdout
        );
        assert!(
            process_tree.stdout.contains("Pid:=2 PPid:=1 comm=envd")
                || process_tree.stdout.contains("PPid:=1 comm=envd"),
            "expected envd as a direct PID 1 child, got: {:?}",
            process_tree.stdout
        );

        let forbidden = sandbox
            .run_command(
                "sh",
                &[
                    "-c",
                    "for proc in /proc/[0-9]*/comm; do cat \"$proc\" 2>/dev/null; done | grep -E '^(systemd|systemd-journal|systemd-udevd|systemd-network|dbus-daemon|runsv|svlogd)$' || true",
                ],
            )
            .await?;
        assert!(
            forbidden.stdout.trim().is_empty(),
            "unexpected guest supervisor processes: {:?}",
            forbidden.stdout
        );

        sandbox.stop().await?;
        Ok(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("test timed out after {:?}", TEST_TIMEOUT))?
}

#[tokio::test]
async fn envd_exit_invalidates_guest_runtime() -> Result<()> {
    common::setup().await;
    tokio::time::timeout(TEST_TIMEOUT, async {
        let sandbox_config = common::default_sandbox_config()?;
        let mut sandbox = FirecrackerSandbox::new(sandbox_config)?;
        sandbox.start().await?;

        let envd_pid = sandbox
            .run_command(
                "sh",
                &[
                    "-c",
                    "for proc in /proc/[0-9]*; do [ \"$(cat \"$proc/comm\" 2>/dev/null)\" = envd ] && basename \"$proc\" && exit 0; done; exit 1",
                ],
            )
            .await?
            .stdout
            .trim()
            .to_owned();
        assert!(!envd_pid.is_empty(), "envd PID should be discoverable");

        let scheduled = sandbox
            .run_command(
                "sh",
                &[
                    "-c",
                    &format!(
                        "(sleep 1; kill -9 {envd_pid}) >/dev/null 2>&1 & echo scheduled"
                    ),
                ],
            )
            .await?;
        assert_eq!(scheduled.stdout.trim(), "scheduled");

        tokio::time::sleep(Duration::from_secs(2)).await;
        let readiness = tokio::time::timeout(
            Duration::from_secs(5),
            SandboxBackend::wait_for_ready(&sandbox),
        )
        .await;
        assert!(
            !matches!(readiness, Ok(Ok(()))),
            "runtime should remain unavailable after envd exits"
        );

        sandbox.stop().await?;
        Ok(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("test timed out after {:?}", TEST_TIMEOUT))?
}

#[tokio::test]
async fn missing_declared_drive_prevents_envd_start() -> Result<()> {
    common::setup().await;
    tokio::time::timeout(TEST_TIMEOUT, async {
        let mut sandbox_config = common::default_sandbox_config()?;
        let boot_args = sandbox_config
            .boot_args
            .take()
            .expect("default sandbox boot arguments");
        sandbox_config.boot_args = Some(format!("{boot_args} agentenv_drives=vdc:/workspace"));
        assert_guest_boot_fails_closed(sandbox_config, "bootstrap failed: mount /dev/vdc").await
    })
    .await
    .map_err(|_| anyhow::anyhow!("test timed out after {:?}", TEST_TIMEOUT))?
}

#[tokio::test]
async fn missing_attached_drive_sub_path_prevents_envd_start() -> Result<()> {
    common::setup().await;
    tokio::time::timeout(TEST_TIMEOUT, async {
        let mut sandbox_config = common::default_sandbox_config()?;
        let image_config_path = sandbox_config
            .common
            .rootfs_image_config
            .as_ref()
            .expect("default sandbox rootfs")
            .image_config_path
            .clone();
        sandbox_config.common.extra_drives = vec![ExtraDrive::try_new_overlaybd_with_mount_path(
            "workspace",
            image_config_path,
            false,
            "/workspace",
            Some("missing/sub-path"),
        )?];
        assert_guest_boot_fails_closed(
            sandbox_config,
            "missing/sub-path: no such file or directory",
        )
        .await
    })
    .await
    .map_err(|_| anyhow::anyhow!("test timed out after {:?}", TEST_TIMEOUT))?
}
