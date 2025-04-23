//! Tasks submitted to a service.

#[cfg(feature = "ord")]
use std::collections::BTreeMap;
#[cfg(not(feature = "ord"))]
use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;

pub mod executor;
pub mod file;

pub use executor::Executor;

/// Task state as defined by the server.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
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
    #[cfg_attr(feature = "serde", serde(rename = "EXECUTOR_ERROR"))]
    ExecutorError,

    /// The task was stopped due to a system error, but not from an Executor,
    /// for example an upload failed due to network issues, the worker's ran out
    /// of disk space, etc.
    #[cfg_attr(feature = "serde", serde(rename = "SYSTEM_ERROR"))]
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
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
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
    pub r#type: file::Type,

    /// The content.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub content: Option<String>,
}

/// An output for a TES task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
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

    /// The type.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub r#type: file::Type,
}

/// Requested resources for a TES task.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(PartialOrd))]
pub struct Resources {
    /// The number of CPU cores.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub cpu_cores: Option<i64>,

    /// Whether or not the task prefers to be preemptible.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub preemptible: Option<bool>,

    /// The amount of RAM (in gigabytes).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ram_gb: Option<f64>,

    /// The amount of disk space (in gigabytes).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub disk_gb: Option<f64>,

    /// The zones.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub zones: Option<Vec<String>>,
}

/// An output file log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
pub struct OutputFileLog {
    /// The URL.
    pub url: String,

    /// The path within the container.
    pub path: String,

    /// The size in bytes.
    pub size_bytes: String,
}

/// A task log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
pub struct TaskLog {
    /// The executor logs.
    pub logs: Vec<executor::Log>,

    /// The start time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub start_time: Option<DateTime<Utc>>,

    /// The end time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub end_time: Option<DateTime<Utc>>,

    /// The output file logs.
    pub outputs: Vec<OutputFileLog>,

    /// The system logs.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub system_logs: Option<Vec<String>>,
}

/// A task.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(PartialOrd))]
pub struct Task {
    /// The ID.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub id: Option<String>,

    /// The current state.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub state: Option<State>,

    /// The user-provided name.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,

    /// The user-provided description.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,

    /// The inputs.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub inputs: Option<Vec<Input>>,

    /// The outputs.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub outputs: Option<Vec<Output>>,

    /// The requested resources.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub resources: Option<Resources>,

    /// The executors.
    pub executors: Vec<Executor>,

    /// The volumes.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub volumes: Option<Vec<String>>,

    /// The tags.
    #[cfg(not(feature = "ord"))]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tags: Option<HashMap<String, String>>,
    /// The tags.
    #[cfg(feature = "ord")]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tags: Option<BTreeMap<String, String>>,

    /// The logs.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub logs: Option<Vec<TaskLog>>,

    /// The time of creation.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub creation_time: Option<DateTime<Utc>>,
}
