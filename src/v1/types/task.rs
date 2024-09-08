//! Tasks submitted to a service.

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use ordered_float::OrderedFloat;
use serde::Deserialize;
use serde::Serialize;

pub mod executor;
pub mod file;

pub use executor::Executor;

/// State of TES task.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum State {
    /// An unknown state.
    #[serde(rename = "UNKNOWN")]
    #[default]
    Unknown,

    /// A queued task.
    #[serde(rename = "QUEUED")]
    Queued,

    /// A task that is initializing.
    #[serde(rename = "INITIALIZING")]
    Initializing,

    /// A task that is running.
    #[serde(rename = "RUNNING")]
    Running,

    /// A task that is paused.
    #[serde(rename = "PAUSED")]
    Paused,

    /// A task that has completed.
    #[serde(rename = "COMPLETE")]
    Complete,

    /// A task that has errored during execution.
    #[serde(rename = "EXECUTOR_ERROR")]
    ExecutorError,

    /// A task that has encountered a system error.
    #[serde(rename = "SYSTEM_ERROR")]
    SystemError,

    /// A task that has been cancelled.
    #[serde(rename = "CANCELED")]
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
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
    #[serde(rename = "type")]
    pub r#type: file::Type,

    /// The content.
    pub content: Option<String>,
}

/// An output for a TES task.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
    #[serde(rename = "type")]
    pub r#type: file::Type,
}

/// Requested resources for a TES task.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputFileLog {
    /// The URL.
    pub url: String,

    /// The path within the container.
    pub path: String,

    /// The size in bytes.
    pub size_bytes: String,
}

/// A task log.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
