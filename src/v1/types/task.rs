//! Tasks submitted to a service.

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use ordered_float::OrderedFloat;

pub mod executor;
pub mod file;

pub use executor::Executor;

/// State of TES task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
pub enum State {
    /// An unknown state.
    #[default]
    Unknown,

    /// A queued task.
    Queued,

    /// A task that is initializing.
    Initializing,

    /// A task that is running.
    Running,

    /// A task that is paused.
    Paused,

    /// A task that has completed.
    Complete,

    /// A task that has errored during execution.
    #[cfg_attr(feature = "serde", serde(rename = "EXECUTOR_ERROR"))]
    ExecutorError,

    /// A task that has encountered a system error.
    #[cfg_attr(feature = "serde", serde(rename = "SYSTEM_ERROR"))]
    SystemError,

    /// A task that has been cancelled.
    Canceled,
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
    pub name: Option<String>,

    /// An optional description.
    pub description: Option<String>,

    /// An optional URL.
    pub url: Option<String>,

    /// Where the input will be mounted within the container.
    pub path: String,

    /// The type.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub r#type: file::Type,

    /// The content.
    pub content: Option<String>,
}

/// An output for a TES task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Output {
    /// An optional name.
    pub name: Option<String>,

    /// An optional description.
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resources {
    /// The number of CPU cores.
    pub cpu_cores: Option<i64>,

    /// Whether or not the task prefers to be preemptible.
    pub preemptible: Option<bool>,

    /// The amount of RAM (in gigabytes).
    pub ram_gb: Option<OrderedFloat<f64>>,

    /// The amount of disk space (in gigabytes).
    pub disk_gb: Option<OrderedFloat<f64>>,

    /// The zones.
    pub zones: Option<Vec<String>>,
}

/// An output file log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
pub struct TaskLog {
    /// The executor logs.
    pub logs: Vec<executor::Log>,

    /// The start time.
    pub start_time: Option<DateTime<Utc>>,

    /// The end time.
    pub end_time: Option<DateTime<Utc>>,

    /// The output file logs.
    pub outputs: Option<Vec<OutputFileLog>>,

    /// The system logs.
    pub system_logs: Option<Vec<String>>,
}

/// A task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Task {
    /// The ID.
    pub id: Option<String>,

    /// The current state.
    pub state: Option<State>,

    /// The user-provided name.
    pub name: Option<String>,

    /// The user-provided description.
    pub description: Option<String>,

    /// The inputs.
    pub inputs: Option<Vec<Input>>,

    /// The outputs.
    pub outputs: Option<Vec<Output>>,

    /// The requested resources.
    pub resources: Option<Resources>,

    /// The executors.
    pub executors: Vec<Executor>,

    /// The volumes.
    pub volumes: Option<Vec<String>>,

    /// The tags.
    pub tags: Option<HashMap<String, String>>,

    /// The logs.
    pub logs: Option<Vec<TaskLog>>,

    /// The time of creation.
    pub creation_time: Option<DateTime<Utc>>,
}
