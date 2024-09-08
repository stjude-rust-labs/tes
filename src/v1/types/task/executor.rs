//! Executor declared within tasks.

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// An executor.
///
/// In short, an executor is a single command that is run in a different
/// container image. [`Executor`]s are run sequentially as they are specified in
/// the task.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
    pub env: Option<HashMap<String, String>>,
}

/// A log for an [`Executor`].
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Log {
    /// The start time.
    pub start_time: Option<DateTime<Utc>>,

    /// The end time.
    pub end_time: Option<DateTime<Utc>>,

    /// The value of the standard output stream.
    pub stdout: Option<String>,

    /// The value of the standard error stream.
    pub stderr: Option<String>,

    /// The exit code.
    pub exit_code: Option<u32>,
}
