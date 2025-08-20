//! Builders for a [`Client`].

use std::time::Duration;

use reqwest::header::HeaderValue;
use reqwest::header::IntoHeaderName;
use url::Url;

use crate::v1::client::Client;

/// An error related to a [`Builder`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A required field was missing from the builder.
    #[error("missing required field `{0}`")]
    Missing(&'static str),

    /// An error from `reqwest`.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// An error related to a URL.
    #[error(transparent)]
    Url(#[from] url::ParseError),
}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [`Client`](Client).
#[derive(Clone, Debug, Default)]
pub struct Builder {
    /// The base URL for the requests.
    url: Option<Url>,

    /// The additional headers to use for requests.
    headers: reqwest::header::HeaderMap,

    /// The connect timeout for the client.
    connect_timeout: Option<Duration>,

    /// The read timeout for the client.
    read_timeout: Option<Duration>,
}

impl Builder {
    /// The default timeout duration for connecting from the client.
    const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);
    /// The default timeout duration for reading from the client.
    const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);

    /// Adds a base URL to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous base URL declarations provided
    /// to the builder.
    pub fn url(mut self, url: impl Into<Url>) -> Self {
        let mut url = url.into();

        // Ensure that the base URL always has a slash.
        if !url.path().ends_with("/") {
            url.set_path(&format!("{}/", url.path()));
        }

        self.url = Some(url);
        self
    }

    /// Attempts to parse a URL and add it as the base URL within the
    /// [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous base URL declarations provided
    /// to the builder.
    pub fn url_from_string(self, url: impl AsRef<str>) -> Result<Self> {
        let url = url.as_ref().parse::<Url>()?;
        Ok(self.url(url))
    }

    /// Inserts a default header for the client within the [`Builder`].
    ///
    /// # Safety
    ///
    /// This method assumes that you will pass only values with the following
    /// conditions (retrieved from the underlying [`HeaderValue`] code that
    /// parses this field):
    ///
    /// Each character in the value:
    ///
    /// * must be printable ASCII _AND_ not be the delete character, _OR_
    /// * must be the tab character (`\t`).
    ///
    /// If your `value` does not conform to this expectation, the function will
    /// panic. This design decision was chosen to avoid needing to unwrap in
    /// the vast majority of cases.
    pub fn insert_header<K>(mut self, key: K, value: impl AsRef<str>) -> Self
    where
        K: IntoHeaderName,
    {
        let value = value.as_ref();
        self.headers.insert::<K>(
            key,
            HeaderValue::from_str(value)
                .unwrap_or_else(|_| panic!("value for header is not allowed: {value}")),
        );
        self
    }

    /// Sets the connect timeout for the client.
    ///
    /// Defaults to 60 seconds.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Sets the read timeout for the client.
    ///
    /// Defaults to 60 seconds.
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    /// Consumes `self` and attempts to build a [`Client`] from the provided
    /// values.
    pub fn try_build(self) -> Result<Client> {
        let url = self.url.map(Ok).unwrap_or(Err(Error::Missing("url")))?;

        let client = reqwest::ClientBuilder::new()
            .connect_timeout(
                self.connect_timeout
                    .unwrap_or(Self::DEFAULT_CONNECT_TIMEOUT),
            )
            .read_timeout(self.read_timeout.unwrap_or(Self::DEFAULT_READ_TIMEOUT))
            .default_headers(self.headers)
            .build()?;

        Ok(Client { url, client })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_adds_a_trailing_slash() {
        let client = Builder::default()
            .url_from_string("http://localhost:4000/v1")
            .unwrap()
            .try_build()
            .unwrap();

        assert_eq!(client.url.as_str(), "http://localhost:4000/v1/");
    }
}
