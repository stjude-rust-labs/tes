//! Tasks submitted to a service.

use std::collections::BTreeMap;

/// A type of an input or output.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum IoType {
    /// A file.
    #[default]
    File,

    /// A directory.
    Directory,
}

/// Task state as defined by the server.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum State {
    /// The state of the task is unknown.
    ///
    /// The cause for this status message may be dependent on the underlying
    /// system.
    ///
    /// The UNKNOWN states provides a safe default for messages where this field
    /// is missing so that a missing field does not accidentally imply that the
    /// state is QUEUED.
    #[default]
    Unknown,

    /// The task is queued and awaiting resources to begin computing.
    Queued,

    /// The task has been assigned to a worker and is currently preparing to
    /// run.
    ///
    /// For example, the worker may be turning on, downloading input files, etc.
    Initializing,

    /// The task is running.
    ///
    /// Input files are downloaded and the first executor has been started.
    Running,

    /// The task is paused.
    ///
    /// The reasons for this would be tied to the specific system running the
    /// job.
    ///
    /// An implementation may have the ability to pause a task, but this is not
    /// required.
    Paused,

    /// The task has completed running.
    ///
    /// Executors have exited without error and output files have been
    /// successfully uploaded.
    Complete,

    /// The task encountered an error in one of the Executor processes.
    ///
    /// Generally, this means that an Executor exited with a non-zero exit code.
    ExecutorError,

    /// The task was stopped due to a system error, but not from an Executor,
    /// for example an upload failed due to network issues, the worker's ran out
    /// of disk space, etc.
    SystemError,

    /// The task was canceled by the user, and downstream resources have been
    /// deleted.
    Canceled,

    /// The task was canceled by the user, but the downstream resources are
    /// still awaiting deletion.
    Canceling,

    /// The task is stopped (preempted) by the system.
    ///
    /// The reasons for this would be tied to the specific system running the
    /// job.
    ///
    /// Generally, this means that the system reclaimed the compute capacity for
    /// reallocation.
    Preempted,
}

impl State {
    /// Returns whether a task is still executing or not.
    pub fn is_executing(&self) -> bool {
        matches!(
            self,
            Self::Unknown | Self::Queued | Self::Initializing | Self::Running | Self::Paused
        )
    }
}

/// An input for a TES task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Input {
    /// An optional name.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,

    /// An optional description.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,

    /// An optional URL.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub url: Option<String>,

    /// Where the input will be mounted within the container.
    pub path: String,

    /// The type.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: IoType,

    /// The content.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub content: Option<String>,

    /// Whether or not the input is streamable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub streamable: Option<bool>,
}

/// An output for a TES task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Output {
    /// An optional name.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,

    /// An optional description.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,

    /// The URL where the result will be stored.
    pub url: String,

    /// The path to the output within the container.
    pub path: String,

    /// The prefix to be removed from matching outputs if `path` contains
    /// wildcards.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub path_prefix: Option<String>,

    /// The type of the output.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: IoType,
}

/// Requested resources for a TES task.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resources {
    /// The number of CPU cores.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub cpu_cores: Option<i32>,

    /// Whether or not the task prefers to be preemptible.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub preemptible: Option<bool>,

    /// The amount of RAM (in gigabytes).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ram_gb: Option<f64>,

    /// The amount of disk space (in gigabytes).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub disk_gb: Option<f64>,

    /// The requested compute zones for the task.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub zones: Option<Vec<String>>,

    /// The optional backend parameters for the task.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub backend_parameters: Option<BTreeMap<String, serde_json::Value>>,

    /// If set to true, backends should fail the task if any backend_parameters
    /// key/values are unsupported, otherwise, backends should attempt to run
    /// the task.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub backend_parameters_strict: Option<bool>,
}

/// An executor.
///
/// In short, an executor is a single command that is run in a different
/// container image. [`Executor`]s are run sequentially as they are specified in
/// the task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Executor {
    /// The image.
    pub image: String,

    /// The command.
    pub command: Vec<String>,

    /// The working directory.
    pub workdir: Option<String>,

    /// The path from which to pipe the standard input stream.
    pub stdin: Option<String>,

    /// The path to pipe the standard output stream to.
    pub stdout: Option<String>,

    /// The path to pipe the standard error stream to.
    pub stderr: Option<String>,

    /// The environment variables.
    pub env: Option<BTreeMap<String, String>>,

    /// Default behavior of running an array of executors is that execution
    /// stops on the first error.
    ///
    /// If `ignore_error` is `true`, then the runner will record error exit
    /// codes, but will continue on to the next executor.
    pub ignore_error: Option<bool>,
}
