//! Responses from a service.

use std::collections::BTreeMap;

use chrono::DateTime;
use chrono::Utc;

use crate::v1::types::task::Executor;
use crate::v1::types::task::Input;
use crate::v1::types::task::Output;
use crate::v1::types::task::Resources;
use crate::v1::types::task::State;

pub mod service_info;

pub use service_info::ServiceInfo;

/// A response from `POST /tasks`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreatedTask {
    /// The ID of the created task.
    pub id: String,
}

/// The response from `GET /tasks`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListTasks<Task> {
    /// The tasks in this page of results.
    pub tasks: Vec<Task>,

    /// The token for the next page of results.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub next_page_token: Option<String>,
}

/// A task output file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutputFile {
    /// The URL.
    pub url: String,

    /// The path within the container.
    pub path: String,

    /// The size in bytes.
    pub size_bytes: String,
}

/// A log for an [`Executor`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutorLog {
    /// The start time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub start_time: Option<DateTime<Utc>>,

    /// The end time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub end_time: Option<DateTime<Utc>>,

    /// The value of the standard output stream.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub stdout: Option<String>,

    /// The value of the standard error stream.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub stderr: Option<String>,

    /// The exit code.
    pub exit_code: i32,
}

/// A task log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskLog {
    /// The executor logs.
    pub logs: Vec<ExecutorLog>,

    /// The log metadata.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub metadata: Option<serde_json::Value>,

    /// The start time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub start_time: Option<DateTime<Utc>>,

    /// The end time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub end_time: Option<DateTime<Utc>>,

    /// The output files.
    pub outputs: Vec<OutputFile>,

    /// The system logs.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub system_logs: Option<Vec<String>>,
}

/// A "minimal" task representation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MinimalTask {
    /// The ID.
    pub id: String,

    /// The current state.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub state: Option<State>,
}

/// A "basic" or "full" task representation.
///
/// Note that a basic representation should always have the following fields as
/// `None`:
///
/// * The `stdout` field of executor logs.
/// * The `stderr` field of executor logs.
/// * The `content` field of inputs.
/// * The `system_logs` field of task logs.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// The executors of the task.
    pub executors: Vec<Executor>,

    /// The volumes of the task.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub volumes: Option<Vec<String>>,

    /// The tags of the task.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tags: Option<BTreeMap<String, String>>,

    /// The logs of the task.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub logs: Option<Vec<TaskLog>>,

    /// The time of creation.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub creation_time: Option<DateTime<Utc>>,
}

/// A generalized representation of a task.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TaskResponse {
    /// A response for when a minimal task representation was requested.
    Minimal(MinimalTask),

    /// A response for when a basic task representation was requested.
    Basic(Task),

    /// A response for when a full task representation was requested.
    Full(Task),
}

impl TaskResponse {
    /// Retrieves a reference to the inner [`MinimalTask`] response if the
    /// variant is [`TaskResponse::Minimal`].
    pub fn as_minimal(&self) -> Option<&MinimalTask> {
        match self {
            Self::Minimal(task) => Some(task),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner [`MinimalTask`] response if the
    /// variant is [`TaskResponse::Minimal`].
    pub fn into_minimal(self) -> Option<MinimalTask> {
        match self {
            Self::Minimal(task) => Some(task),
            _ => None,
        }
    }

    /// Retrieves a reference to the inner [`Task`] response if the variant
    /// is [`TaskResponse::Basic`] or [`TaskResponse::Full`].
    pub fn as_task(&self) -> Option<&Task> {
        match self {
            Self::Basic(task) | Self::Full(task) => Some(task),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner [`Task`] response if the variant
    /// is [`TaskResponse::Basic`] or [`TaskResponse::Full`].
    pub fn into_task(self) -> Option<Task> {
        match self {
            Self::Basic(task) | Self::Full(task) => Some(task),
            _ => None,
        }
    }
}
