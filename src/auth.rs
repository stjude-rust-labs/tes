//! Implementation of authorization for TES clients.

use std::future::ready;

use futures::FutureExt;
use futures::future::BoxFuture;
use reqwest::header::HeaderValue;

#[cfg(feature = "oauth")]
pub mod oauth;

/// Export the request builder as it's part of the `Authorizer`` trait method
/// signatures.
pub use reqwest::RequestBuilder;

/// Represents an authorization error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An OAuth error occurred.
    #[cfg(feature = "oauth")]
    #[error(transparent)]
    OAuth(#[from] oauth::Error),
    /// Reauthorization was declined.
    #[error("reauthorization was declined because no refresh token was available")]
    ReauthorizationDeclined,
}

/// Represents a generic authorizer of TES requests.
pub trait Authorizer: Send + Sync {
    /// Authorizes for the given request.
    ///
    /// On the first attempt of each request, `rejected` will be `None`.
    ///
    /// If the request fails with a 401 status, `authorize` is called again with
    /// `rejected` set to the `Authorization` header used on the previous
    /// attempt.
    ///
    /// Only authorizers that return `true` for `reauthorizes` will be called
    /// again.
    fn authorize<'a>(
        &'a self,
        request: RequestBuilder,
        rejected: Option<&'a HeaderValue>,
    ) -> BoxFuture<'a, Result<RequestBuilder, Error>>;

    /// Determines if the authorizer supports reauthorization.
    ///
    /// Returns `true` if it does or `false` if it does not.
    ///
    /// The default implementation of this method is to return `false`.
    fn reauthorizes(&self) -> bool {
        false
    }
}

/// Implements an [`Authorizer`] for basic HTTP authorization.
pub struct BasicAuthorizer {
    /// The user name for the authorization.
    username: String,
    /// The password for the authorization.
    password: Option<String>,
}

impl BasicAuthorizer {
    /// Constructs a new [`BasicAuthorizer`] from the given username and
    /// password.
    pub fn new(username: impl Into<String>, password: Option<impl Into<String>>) -> Self {
        Self {
            username: username.into(),
            password: password.map(Into::into),
        }
    }
}

impl Authorizer for BasicAuthorizer {
    fn authorize<'a>(
        &'a self,
        request: RequestBuilder,
        _rejected: Option<&'a HeaderValue>,
    ) -> BoxFuture<'a, Result<RequestBuilder, Error>> {
        ready(Ok(
            request.basic_auth(&self.username, self.password.as_ref())
        ))
        .boxed()
    }
}

/// Implements an [`Authorizer`] for bearer token HTTP authorization.
pub struct BearerTokenAuthorizer(String);

impl BearerTokenAuthorizer {
    /// Constructs a new [`BearerTokenAuthorizer`] from the given token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

impl Authorizer for BearerTokenAuthorizer {
    fn authorize<'a>(
        &'a self,
        request: RequestBuilder,
        _rejected: Option<&'a HeaderValue>,
    ) -> BoxFuture<'a, Result<RequestBuilder, Error>> {
        ready(Ok(request.bearer_auth(&self.0))).boxed()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use mockito::Server;
    use reqwest::Client;
    use reqwest::Method;

    use super::*;

    #[tokio::test]
    async fn test_basic_authorizer() {
        let mut server = Server::new_async().await;

        let url = server.url();
        let endpoint = server
            .mock("GET", "/")
            .match_header("authorization", "Basic Zm9vOmJhcg==")
            .with_status(200)
            .create();

        let authorizer = BasicAuthorizer::new("foo", Some("bar"));
        assert!(!authorizer.reauthorizes());

        let client = Client::new();
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(request, None).await.unwrap();
        request.send().await.unwrap();

        endpoint.assert();
    }

    #[tokio::test]
    async fn test_basic_authorizer_no_password() {
        let mut server = Server::new_async().await;

        let url = server.url();
        let endpoint = server
            .mock("GET", "/")
            .match_header("authorization", "Basic Zm9vOg==")
            .with_status(200)
            .create();

        let authorizer = BasicAuthorizer::new("foo", None::<&str>);
        assert!(!authorizer.reauthorizes());

        let client = Client::new();
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(request, None).await.unwrap();
        request.send().await.unwrap();

        endpoint.assert();
    }

    #[tokio::test]
    async fn test_bearer_token_authorizer() {
        let mut server = Server::new_async().await;

        let url = server.url();
        let endpoint = server
            .mock("GET", "/")
            .match_header("authorization", "Bearer token")
            .with_status(200)
            .create();

        let authorizer = BearerTokenAuthorizer::new("token");
        assert!(!authorizer.reauthorizes());

        let client = Client::new();
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(request, None).await.unwrap();
        request.send().await.unwrap();

        endpoint.assert();
    }
}
