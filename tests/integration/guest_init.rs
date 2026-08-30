use crate::common;

use agentenv::sandbox::{FirecrackerSandbox, SandboxBackend, SandboxExecutor};
use anyhow::Result;
use tokio::time::Duration;

const TEST_TIMEOUT: Duration = Duration::from_secs(90);

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
