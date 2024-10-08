//! Responses from a service.

pub mod service_info;
pub mod task;

pub use service_info::ServiceInfo;

/// A response from `POST /tasks`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
pub struct CreateTask {
    /// The ID of the created task.
    pub id: String,
}

/// The response from `GET /tasks`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
pub struct ListTasks<Task> {
    /// The tasks in this page of results.
    pub tasks: Vec<Task>,

    /// The token for the next page of results.
    pub next_page_token: Option<String>,
}
