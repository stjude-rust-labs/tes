//! Files declared within tasks.

/// A type of file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ord", derive(Ord, PartialOrd))]
pub enum Type {
    /// A file.
    #[cfg_attr(feature = "serde", serde(rename = "FILE"))]
    #[default]
    File,

    /// A directory.
    #[cfg_attr(feature = "serde", serde(rename = "DIRECTORY"))]
    Directory,
}
