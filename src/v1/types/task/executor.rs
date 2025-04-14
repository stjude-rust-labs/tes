//! Executor declared within tasks.

#[cfg(feature = "ord")]
use std::collections::BTreeMap;
#[cfg(not(feature = "ord"))]
use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;

/// An executor.
///
/// In short, an executor is a single command that is run in a different
/// container image. [`Executor`]s are run sequentially as they are specified in
/// the task.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
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
    #[cfg(not(feature = "ord"))]
    pub env: Option<HashMap<String, String>>,
    /// The environment variables.
    #[cfg(feature = "ord")]
    pub env: Option<BTreeMap<String, String>>,

    /// Default behavior of running an array of executors is that execution
    /// stops on the first error.
    ///
    /// If `ignore_error` is `true`, then the runner will record error exit
    /// codes, but will continue on to the next executor.
    pub ignore_error: Option<bool>,
}

/// A log for an [`Executor`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
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
