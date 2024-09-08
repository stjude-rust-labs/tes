//! Options for a [`Client`](super::Client).

use reqwest::header::HeaderMap;

/// The number of retries to the server by default.
const DEFAULT_RETRIES: u32 = 3;

/// Options used within a [`Client`](super::Client).
#[derive(Debug)]
pub struct Options {
    /// Headers to include in each request.
    pub headers: HeaderMap,

    /// The number of retries per request.
    pub retries: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            headers: Default::default(),
            retries: DEFAULT_RETRIES,
        }
    }
}
