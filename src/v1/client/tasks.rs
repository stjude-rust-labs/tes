//! Task-related entities used within a client.

use serde::Deserialize;
use serde::Serialize;

/// An argument that affects which fields are returned on certain task-related
/// endpoints.

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum View {
    /// Only includes the `id` and `state` fields in the returned task.
    #[default]
    Minimal,

    /// Includes all available fields except:
    ///
    /// * Logs for stdout (`tesTask.ExecutorLog.stdout`).
    /// * Logs for stderr (`tesTask.ExecutorLog.stderr`).
    /// * The content of inputs (`tesInput.content`).
    /// * The system logs (`tesTaskLog.system_logs`).
    Basic,

    /// Includes all fields.
    Full,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            View::Minimal => write!(f, "MINIMAL"),
            View::Basic => write!(f, "BASIC"),
            View::Full => write!(f, "FULL"),
        }
    }
}
