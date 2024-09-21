//! Responses related to tasks.

use crate::v1::types::Task;
use crate::v1::types::task::State;

/// A response for when `?view=MINIMAL` in a task endpoint.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MinimalTask {
    /// The ID.
    pub id: String,

    /// The current state.
    pub state: Option<State>,
}

/// A generalized response for getting tasks with the `view` parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Response {
    /// A response for when `?view=MINIMAL` in a task endpoint.
    Minimal(MinimalTask),

    /// A response for when `?view=BASIC` in a task endpoint.
    ///
    /// **NOTE:** all of the fields that are optional in the specification for
    /// this response are also optional on [`Task`], so we can reuse this
    /// struct here instead of subsetting it.
    Basic(Task),

    /// A response for when `?view=FULL` in a task endpoint.
    Full(Task),
}

impl Response {
    /// Retrieves a reference to the inner [`MinimalTask`] response if the
    /// variant is [`Response::Minimal`].
    pub fn as_minimal(&self) -> Option<&MinimalTask> {
        match self {
            Response::Minimal(task) => Some(task),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner [`MinimalTask`] response if the
    /// variant is [`Response::Minimal`].
    pub fn into_minimal(self) -> Option<MinimalTask> {
        match self {
            Response::Minimal(task) => Some(task),
            _ => None,
        }
    }

    /// Retrieves a reference to the inner [`Task`] response if the variant
    /// is [`Response::Basic`] or [`Response::Full`].
    pub fn as_task(&self) -> Option<&Task> {
        match self {
            Response::Basic(task) | Response::Full(task) => Some(task),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner [`Task`] response if the variant
    /// is [`Response::Basic`] or [`Response::Full`].
    pub fn into_task(self) -> Option<Task> {
        match self {
            Response::Basic(task) | Response::Full(task) => Some(task),
            _ => None,
        }
    }
}
