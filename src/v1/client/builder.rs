//! Builders for a [`Client`].

use reqwest::header::HeaderValue;
use reqwest::header::IntoHeaderName;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use url::Url;

use crate::v1::client::Client;
use crate::v1::client::Options;

/// An error related to a [`Builder`].
#[derive(Debug)]
pub enum Error {
    /// A required field was missing from the builder.
    Missing(&'static str),

    /// An error from `reqwest`.
    Reqwest(reqwest::Error),

    /// An error related to a URL.
    Url(url::ParseError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => write!(f, "missing required field: {field}"),
            Error::Reqwest(err) => write!(f, "reqwest error: {err}"),
            Error::Url(err) => write!(f, "url error: {err}"),
        }
    }
}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [`Client`](Client).
#[derive(Clone, Debug, Default)]
pub struct Builder {
    /// The base URL for the requests.
    url: Option<Url>,

    /// The options passed to the client.
    options: Options,
}

impl Builder {
    /// Adds a base URL to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous base URL declarations provided
    /// to the builder.
    pub fn url(mut self, url: impl Into<Url>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Attempts to parse a URL and add it as the base URL within the
    /// [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous base URL declarations provided
    /// to the builder.
    pub fn url_from_string(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.url = Some(url.as_ref().parse::<Url>().map_err(Error::Url)?);
        Ok(self)
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
        self.options.headers.insert::<K>(
            key,
            HeaderValue::from_str(value)
                .unwrap_or_else(|_| panic!("value for header is not allowed: {value}")),
        );
        self
    }

    /// Sets the maximum retries for the client within the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous maximum retry declarations
    /// provided to the builder.
    pub fn retries(mut self, value: u32) -> Self {
        self.options.retries = value;
        self
    }

    /// Consumes `self` and attempts to build a [`Client`] from the provided
    /// values.
    pub fn try_build(self) -> Result<Client> {
        let url = self.url.map(Ok).unwrap_or(Err(Error::Missing("url")))?;

        let client = reqwest::ClientBuilder::new()
            .default_headers(self.options.headers)
            .build()
            .map_err(Error::Reqwest)?;

        let client = reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(
                ExponentialBackoff::builder().build_with_max_retries(self.options.retries),
            ))
            .build();

        Ok(Client { url, client })
    }
}
