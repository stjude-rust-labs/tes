//! Files declared within tasks.

use serde::Deserialize;
use serde::Serialize;

/// A type of file.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Type {
    /// A file.
    #[serde(rename = "FILE")]
    #[default]
    File,

    /// A directory.
    #[serde(rename = "DIRECTORY")]
    Directory,
}
