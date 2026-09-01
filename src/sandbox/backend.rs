//! Abstractions for sandbox backends.
//!
//! [`SandboxBackend`] represents the lifecycle of a single sandbox instance.
//! [`SandboxBackendFactory`] is responsible for constructing new sandbox
//! instances (from scratch, from a committed snapshot, or from paused state).

use std::any::Any;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{
    EnvdAccessToken, Executor, FreshSandboxBuildSpec, ProcessHandle, ProcessOpts, ProcessOutput,
    SandboxLaunchConfig, SandboxNetworkPolicy,
};
use crate::sandbox::CustomExtensionParams;
use crate::snapshot::RunnableSnapshot;
use crate::types::SandboxId;

/// A concrete sandbox backend's paused state.
///
/// The Orchestrator treats this value as completely opaque: it stores it in
/// [`SandboxMetadata`][crate::orchestrator::SandboxMetadata] after a
/// `pause` call and passes it back to
/// [`SandboxBackendFactory::build_from_paused_state`] when a resume is requested.
/// Concrete implementations own their serialized form.
pub trait PausedSandboxState: Any + fmt::Debug + Send + Sync + 'static {
    fn encode(&self) -> Result<Value>;

    /// Local artifacts this paused sandbox will reopen on resume.
    /// The orchestrator only carries this value to the image-liveness layer; it
    /// does not interpret the backend-specific artifact identities inside it.
    fn runtime_artifacts(&self) -> RuntimeArtifactSet;
    /// Effective envd control-plane port persisted with the paused runtime, when available.
    fn control_plane_port(&self) -> Option<u16> {
        None
    }
}

impl dyn PausedSandboxState {
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: PausedSandboxState,
    {
        (self as &dyn Any).downcast_ref::<T>()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SandboxCaptureError {
    #[error("{0}")]
    Recoverable(#[source] anyhow::Error),
    #[error("{0}")]
    Terminal(#[source] anyhow::Error),
}

impl SandboxCaptureError {
    pub fn recoverable(err: anyhow::Error) -> Self {
        Self::Recoverable(err)
    }

    pub fn terminal(err: anyhow::Error) -> Self {
        Self::Terminal(err)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

impl From<anyhow::Error> for SandboxCaptureError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<Self>() {
            Ok(snapshot_err) => snapshot_err,
            Err(err) => Self::Recoverable(err),
        }
    }
}

pub type SandboxCaptureResult<T> = std::result::Result<T, SandboxCaptureError>;
pub type SandboxForkResult = anyhow::Result<Box<dyn SandboxBackend>>;

/// State in which a snapshot capture must leave its source sandbox.
///
/// `LeavePaused` is useful when the captured snapshot is about to be moved or
/// published and no guest execution may occur after the capture point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxSnapshotSourceDisposition {
    Resume,
    LeavePaused,
}

#[derive(Clone, Debug)]
pub struct SandboxForkSpec {
    pub sandbox_id: SandboxId,
    pub envd_access_token: Option<EnvdAccessToken>,
}

/// Opaque set of local runtime artifacts a sandbox needs while it is alive.
///
/// Sandbox backends construct this from their runtime config, the orchestrator
/// carries it across lifecycle boundaries, and the image-liveness layer decides
/// how to protect the concrete local artifacts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeArtifactSet {
    overlaybd_image_config_paths: Vec<PathBuf>,
}

impl RuntimeArtifactSet {
    /// No local runtime artifacts.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from overlaybd image configs whose local-only layers must stay
    /// available while the sandbox may reopen them.
    pub(crate) fn from_overlaybd_image_configs(overlaybd_image_config_paths: Vec<PathBuf>) -> Self {
        Self {
            overlaybd_image_config_paths,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.overlaybd_image_config_paths.is_empty()
    }

    pub(crate) fn into_overlaybd_image_config_paths(self) -> Vec<PathBuf> {
        self.overlaybd_image_config_paths
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SandboxRuntimeInfo {
    pub rootfs_virtual_size: Option<u64>,
    pub runtime_artifacts: RuntimeArtifactSet,
}

/// Opaque captured snapshot artifacts produced from a running sandbox.
///
/// Unlike [`PausedSandboxState`], this value is intended for one-shot
/// consumption by snapshot publication code. Concrete backends may use it to
/// keep temporary artifact directories alive until publication finishes.
pub struct CapturedSandboxSnapshot {
    inner: Box<dyn Any + Send>,
}

impl CapturedSandboxSnapshot {
    pub fn new<T>(snapshot: T) -> Self
    where
        T: Send + 'static,
    {
        Self {
            inner: Box::new(snapshot),
        }
    }

    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Send + 'static,
    {
        self.inner.downcast_ref::<T>()
    }

    pub fn downcast<T>(self) -> std::result::Result<T, Self>
    where
        T: Send + 'static,
    {
        match self.inner.downcast::<T>() {
            Ok(inner) => Ok(*inner),
            Err(inner) => Err(Self { inner }),
        }
    }
}

impl fmt::Debug for CapturedSandboxSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapturedSandboxSnapshot")
            .field("opaque", &true)
            .finish()
    }
}

/// Backend request for capturing a running sandbox.
///
/// The artifact root is only meaningful for `LeavePaused`: it lets the
/// orchestrator place the resumable source state under its durable lifecycle
/// store. When it is absent, the backend owns the artifact lifetime.
#[derive(Clone, Copy, Debug)]
pub struct SandboxSnapshotCaptureRequest<'a> {
    source_disposition: SandboxSnapshotSourceDisposition,
    artifact_root: Option<&'a Path>,
}

impl<'a> SandboxSnapshotCaptureRequest<'a> {
    pub fn resume_source() -> Self {
        Self {
            source_disposition: SandboxSnapshotSourceDisposition::Resume,
            artifact_root: None,
        }
    }

    pub fn leave_source_paused(artifact_root: Option<&'a Path>) -> Self {
        Self {
            source_disposition: SandboxSnapshotSourceDisposition::LeavePaused,
            artifact_root,
        }
    }

    pub fn source_disposition(self) -> SandboxSnapshotSourceDisposition {
        self.source_disposition
    }

    pub fn artifact_root(self) -> Option<&'a Path> {
        self.artifact_root
    }
}

/// Successful snapshot capture and the resulting source-sandbox state.
///
/// Encoding the state in the outcome prevents callers from mistaking a paused
/// source for a running one or from discarding the state needed to resume it.
#[derive(Debug)]
pub enum SandboxSnapshotCaptureOutcome {
    SourceRunning {
        captured_snapshot: CapturedSandboxSnapshot,
    },
    SourcePaused {
        captured_snapshot: CapturedSandboxSnapshot,
        paused_state: Arc<dyn PausedSandboxState>,
    },
}

/// Lifecycle interface for a single sandbox instance.
///
/// Implementors must be `Send + 'static` so that they can be stored inside
/// `Arc<Mutex<Box<dyn SandboxBackend>>>` handles managed by the Orchestrator.
#[async_trait]
pub trait SandboxBackend: Send + 'static {
    /// Start the sandbox and block until readiness.
    async fn start(&mut self) -> Result<()>;

    /// Start the sandbox without waiting for the sandbox to become ready.
    async fn start_nowait(&mut self) -> Result<()>;

    /// Block until the sandbox signals readiness.
    ///
    /// Should be called after [`start_nowait`][Self::start_nowait] before any
    /// workload is submitted.
    async fn wait_for_ready(&self) -> Result<()>;

    /// Resume a paused but not-yet-stopped sandbox from its snapshot.
    ///
    /// Idempotent: calling `resume` more than once must not return an error.
    async fn resume(&mut self) -> Result<()>;

    /// Capture a persistent snapshot from a running sandbox and leave the
    /// source in the state selected by `request`.
    ///
    /// A successful `LeavePaused` capture must return the exact paused state
    /// represented by the captured snapshot. The caller is then expected to
    /// persist that state before invoking [`stop`][Self::stop]. A successful
    /// `Resume` capture must return only after the source is running again.
    ///
    /// [`SandboxCaptureError::Terminal`] indicates snapshot capture mutated the live
    /// runtime before failing, so callers must not keep treating the sandbox
    /// as safely runnable.
    /// [`SandboxCaptureError::Recoverable`] must guarantee the source has been
    /// restored to a running state before the error is returned, regardless of
    /// the requested source disposition.
    async fn capture_snapshot(
        &mut self,
        request: SandboxSnapshotCaptureRequest<'_>,
    ) -> SandboxCaptureResult<SandboxSnapshotCaptureOutcome>;

    /// Fork this running sandbox into ready child backends.
    ///
    /// The outer error is reserved for failures before child startup begins.
    /// After the source has been restored, implementations must attempt every
    /// child concurrently and return one result per `spec` entry in the
    /// same order. Successful children stay running when a sibling fails.
    ///
    /// [`SandboxCaptureError::Terminal`] indicates the fork attempt mutated the
    /// source runtime past safe resume, so callers must stop treating the
    /// source as runnable. Child construction/start failures after source
    /// recovery belong in the corresponding [`SandboxForkResult`].
    async fn fork(
        &mut self,
        spec: &[SandboxForkSpec],
    ) -> SandboxCaptureResult<Vec<SandboxForkResult>>;

    /// Stop the sandbox and release all associated system resources.
    ///
    /// Idempotent: calling `stop` more than once must not return an error.
    async fn stop(&mut self) -> Result<()>;

    /// Obtain the IP address that the sandbox can use to interact with the host.
    fn host_interaction_ip(&self) -> Option<std::net::Ipv4Addr>;

    /// Return runtime facts that are only known after the backend has started.
    fn runtime_info(&self) -> SandboxRuntimeInfo;

    /// Local runtime artifacts this sandbox opens on start.
    fn startup_artifacts(&self) -> RuntimeArtifactSet;

    /// Update the sandbox network policy at runtime.
    async fn update_network_policy(&mut self, policy: Option<SandboxNetworkPolicy>) -> Result<()>;

    /// Update the custom extension params held by the sandbox runtime.
    ///
    /// Plain assignment of an already-approved value: the custom extension
    /// patch-params hook is invoked by the caller (orchestrator layer), not
    /// by the backend. Cannot fail.
    fn update_custom_extension_params(&mut self, params: Option<CustomExtensionParams>);
}

/// Factory interface for creating and restoring sandbox backend instances.
///
/// A single factory instance is stored inside the
/// [`Orchestrator`][crate::orchestrator::Orchestrator] and is used for every
/// `create_sandbox` and `resume_sandbox` request.
pub trait SandboxBackendFactory: Send + Sync + 'static {
    /// Build a brand-new sandbox backend from a high-level launch request.
    fn build(
        &self,
        build_spec: FreshSandboxBuildSpec,
        launch_config: SandboxLaunchConfig,
    ) -> Result<Box<dyn SandboxBackend>>;

    /// Build a sandbox backend from a runnable committed snapshot plus launch request.
    fn build_from_snapshot(
        &self,
        snapshot: &RunnableSnapshot,
        launch_config: SandboxLaunchConfig,
    ) -> Result<Box<dyn SandboxBackend>>;

    /// Decode backend-specific paused state loaded from persistence.
    fn decode_paused_state(
        &self,
        artifact_root: PathBuf,
        state: Value,
    ) -> Result<Arc<dyn PausedSandboxState>>;

    /// Build a sandbox backend from backend-specific paused snapshot state.
    fn build_from_paused_state(
        &self,
        sandbox_id: crate::types::SandboxId,
        state: &dyn PausedSandboxState,
        envd_access_token: Option<EnvdAccessToken>,
    ) -> Result<Box<dyn SandboxBackend>>;
}

/// Process execution capability of a running sandbox.
///
/// Implement [`executor`][Self::executor] to provide a [`ProcessClient`][envd::process::ProcessClient]-backed
/// [`Executor`]. The three convenience methods (`run_command`,
/// `run_command_with_opts`, `start_process`) have default implementations that
/// simply call `self.executor()?` and delegate, so callers can continue using
/// the familiar `sandbox.run_command(...)` pattern without boilerplate.
///
/// # Note on `Send`
/// `&Self` may be `!Send` (e.g. `FirecrackerSandbox` holds tonic clients that
/// are `!Sync`), so the generated futures are not required to be `Send`.
#[async_trait(?Send)]
pub trait SandboxExecutor: Send {
    /// Obtain a process executor backed by this sandbox's envd connection.
    ///
    /// Returns an error if the sandbox is not running.
    fn executor(&self) -> Result<Executor<'_>>;

    /// Run a command inside the sandbox and wait for it to complete.
    ///
    /// Returns the captured stdout, stderr, and exit code.
    ///
    /// # Example
    /// ```no_run
    /// use agentenv::sandbox::SandboxExecutor;
    /// # async fn example(sandbox: &impl SandboxExecutor) -> anyhow::Result<()> {
    /// let output = sandbox.run_command("echo", &["hello", "world"]).await?;
    /// assert_eq!(output.exit_code, 0);
    /// println!("{}", output.stdout);
    /// # Ok(())
    /// # }
    /// ```
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<ProcessOutput> {
        self.executor()?.run_command(cmd, args).await
    }

    /// Run a command with custom options and wait for it to complete.
    ///
    /// # Example
    /// ```no_run
    /// use agentenv::sandbox::{ProcessOpts, SandboxExecutor};
    /// use std::collections::HashMap;
    /// # async fn example(sandbox: &impl SandboxExecutor) -> anyhow::Result<()> {
    /// let opts = ProcessOpts::new().with_cwd("/tmp");
    /// let output = sandbox.run_command_with_opts("ls", &["-la"], &opts).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn run_command_with_opts(
        &self,
        cmd: &str,
        args: &[&str],
        opts: &ProcessOpts,
    ) -> Result<ProcessOutput> {
        self.executor()?
            .run_command_with_opts(cmd, args, opts)
            .await
    }

    /// Create a directory (and any missing parents) inside the sandbox.
    ///
    /// Goes through envd's filesystem service rather than exec'ing a binary,
    /// so it works in images that ship no userland (scratch, distroless).
    /// An already-existing directory is not an error.
    ///
    /// # Example
    /// ```no_run
    /// use agentenv::sandbox::SandboxExecutor;
    /// # async fn example(sandbox: &impl SandboxExecutor) -> anyhow::Result<()> {
    /// sandbox.create_dir_all("/home/user/work").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create_dir_all(&self, path: &str) -> Result<()> {
        self.executor()?.create_dir_all(path).await
    }

    /// Start a long-running process and return a handle.
    ///
    /// # Example
    /// ```no_run
    /// use agentenv::sandbox::{ProcessOpts, SandboxExecutor};
    /// # async fn example(sandbox: &impl SandboxExecutor) -> anyhow::Result<()> {
    /// let mut handle = sandbox.start_process("cat", &[], &ProcessOpts::default()).await?;
    /// handle.send_stdin(b"hello\n").await?;
    /// handle.kill().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn start_process(
        &self,
        cmd: &str,
        args: &[&str],
        opts: &ProcessOpts,
    ) -> Result<ProcessHandle> {
        self.executor()?.start_process(cmd, args, opts).await
    }
}
