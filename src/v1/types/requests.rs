//! Requests to a service.

use std::collections::BTreeMap;

use crate::v1::types::task::Executor;
use crate::v1::types::task::Input;
use crate::v1::types::task::Output;
use crate::v1::types::task::Resources;
use crate::v1::types::task::State;

/// A requested view of tasks.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum View {
    /// The `MINIMAL` view.
    #[default]
    Minimal,

    /// The `BASIC` view.
    Basic,

    /// The `FULL` view.
    Full,
}

/// A method to get the default view of parameters.
const fn default_view() -> View {
    View::Minimal
}

/// The query parameters for `GetTask` endpoint.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GetTaskParams {
    /// The view of the returned task.
    #[cfg_attr(feature = "serde", serde(default = "default_view"))]
    pub view: View,
}

/// The query parameters for `ListTasks` endpoint.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListTasksParams {
    /// The filter for task name (prefixed).
    ///
    /// If unspecified, no task name filtering is done.
    pub name_prefix: Option<String>,
    /// The filter for task state.
    ///
    /// If unspecified, no task state filtering is done.
    pub state: Option<State>,
    /// The filter for task tag keys.
    ///
    /// This is zipped with `tag_values`.
    ///
    /// If empty, no task tags filtering is done.
    #[cfg_attr(feature = "serde", serde(rename = "tag_key", default))]
    pub tag_keys: Vec<String>,
    /// The filter for task tag values.
    ///
    /// This is zipped with `tag_keys`.
    ///
    /// If the value is empty, it matches all values.
    ///
    /// It is an error if more values are supplied than keys.
    #[cfg_attr(feature = "serde", serde(rename = "tag_value", default))]
    pub tag_values: Vec<String>,
    /// The number of tasks to return in one page.
    ///
    /// Must be less than 2048. Defaults to 256.
    pub page_size: Option<u16>,
    /// The page token to retrieve the next page of results.
    ///
    /// If unspecified, returns the first page of results.
    ///
    /// The value can be found in the `next_page_token`` field of the last
    /// returned result of `list_tasks`.
    pub page_token: Option<String>,
    /// The view of the returned task(s).
    #[cfg_attr(feature = "serde", serde(default = "default_view"))]
    pub view: View,
}

/// Represents the request body of the `CreateTask` endpoint.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Task {
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
}
