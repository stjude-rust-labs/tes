//! Responses from a service.

mod service;
pub mod task;

use serde::Deserialize;
use serde::Serialize;
pub use service::ServiceInfo;

/// A response from `POST /tasks`.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateTask {
    /// The ID of the created task.
    pub id: String,
}

/// The response from `GET /tasks`.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ListTasks<Task> {
    /// The tasks in this page of results.
    pub tasks: Vec<Task>,

    /// The token for the next page of results.
    pub next_page_token: Option<String>,
}
