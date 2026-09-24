//! Implementation of authorization for TES clients.

use std::future::ready;
use std::sync::Arc;

use futures::FutureExt;
use futures::future::BoxFuture;
use oauth2::AccessToken;
use oauth2::RefreshToken;
use oauth2::StandardDeviceAuthorizationResponse;
use reqwest::RequestBuilder;
use tokio::sync::Mutex;

use crate::oauth;

/// Represents an authorization error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An OAuth error occurred.
    #[error(transparent)]
    Oauth(#[from] oauth::Error),
    /// No refresh token was provided the client.
    #[error(
        "the client did not receive a refresh token and could not automatically refresh the \
         access token"
    )]
    NoRefreshToken,
}

/// Represents a generic authorizer of TES requests.
pub trait Authorizer: Send + Sync {
    /// Authorizes for the given request.
    ///
    /// On the first attempt of each request, `initial` will be `true`.
    ///
    /// If the service responds with a 401 status and the authorizer supports
    /// reauthorization, this method will be called one more time with
    /// `initial` set to `false`.
    fn authorize<'a>(
        &'a self,
        initial: bool,
        request: RequestBuilder,
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
        _initial: bool,
        request: RequestBuilder,
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
        _initial: bool,
        request: RequestBuilder,
    ) -> BoxFuture<'a, Result<RequestBuilder, Error>> {
        ready(Ok(request.bearer_auth(&self.0))).boxed()
    }
}

/// Collection of OAuth-related tokens.
struct Tokens {
    /// The OAuth access token.
    ///
    /// This ends up as the bearer token for authorized requests.
    access: AccessToken,
    /// The OAuth refresh token.
    ///
    /// If `Some`, the token is used to automatically refresh the access token
    /// upon expiration.
    refresh: Option<RefreshToken>,
}

/// The type of the prompt callback function.
type PromptFn = dyn Fn(&StandardDeviceAuthorizationResponse) + Send + Sync;
/// The type of the reauthorization callback function.
type ReauthFn = dyn Fn(Option<&oauth::Error>) -> bool + Send + Sync;

/// Implements an [`Authorizer`] for OAuth authorization.
pub struct OAuthAuthorizer {
    /// The OAuth configuration.
    config: oauth::Config,
    /// The current tokens of the authorizer.
    tokens: Arc<Mutex<Option<Tokens>>>,
    /// The prompt callback for displaying a device code to the user.
    prompt: Box<PromptFn>,
    /// The reauth callback.
    reauth: Option<Box<ReauthFn>>,
}

impl OAuthAuthorizer {
    /// Constructs a new [`OAuthAuthorizer`] with the given configuration and
    /// device code prompt callback.
    ///
    /// The device code prompt callback is invoked when a URI and device code
    /// has been obtained for the user to authorize the client.
    pub fn new<P>(config: oauth::Config, prompt: P) -> Self
    where
        P: Fn(&StandardDeviceAuthorizationResponse) + Send + Sync + 'static,
    {
        Self {
            config,
            tokens: Arc::new(Mutex::new(None)),
            prompt: Box::new(prompt),
            reauth: None,
        }
    }

    /// Sets the reauthorization handler.
    ///
    /// The reauthorization handler is invoked prior to reauthorizing a
    /// previously authorized device.
    ///
    /// Reauthorization may occur if there were no refresh token given to the
    /// client or if a token refresh operation failed.
    ///
    /// The provided error will be `Some` if the token refresh operation failed.
    ///
    /// If the handler returns `true`, a new device code authorization flow is
    /// attempted.
    ///
    /// If the handler returns `false`, an error is returned as the result of
    /// authorization.
    pub fn on_reauthorization<E>(mut self, reauth: E) -> Self
    where
        E: Fn(Option<&oauth::Error>) -> bool + Send + Sync + 'static,
    {
        self.reauth = Some(Box::new(reauth));
        self
    }

    /// Performs an initial authorization.
    ///
    /// This is a no-op if the authorizer has already acquired an access token.
    ///
    /// Returns `true` if the authorizer can refresh the access token or `false`
    /// if no refresh token was returned.
    pub async fn initialize(&self) -> Result<bool, Error> {
        self.perform_authorization(true, None).await.map(|(r, _)| r)
    }

    /// Performs authorization.
    ///
    /// If the provided request is `Some`, the `Authorization` header is set for
    /// the request upon successful authorization.
    ///
    /// Upon success, returns a tuple of whether or not there is a refresh token
    /// and the provided request.
    async fn perform_authorization(
        &self,
        initial: bool,
        mut request: Option<RequestBuilder>,
    ) -> Result<(bool, Option<RequestBuilder>), Error> {
        // Take a lock on the tokens for the entire OAuth operation.
        // This is intentionally a long-lived lock as at most one
        // authorization flow should occur at a time.
        let mut tokens = self.tokens.lock().await;

        // If this is the initial request and we have an access token, use it.
        if initial && let Some(tokens) = tokens.as_ref() {
            return Ok((
                tokens.refresh.is_some(),
                request.map(|r| r.bearer_auth(tokens.access.secret())),
            ));
        }

        // If this is not the initial request, attempt to refresh the access
        // token if it is possible to do so.
        if !initial {
            if let Some(refresh) = tokens.as_ref().and_then(|tokens| tokens.refresh.as_ref()) {
                // Refresh the token
                match oauth::refresh_token(&self.config, refresh).await {
                    Ok((access, refresh)) => {
                        let has_refresh = refresh.is_some();
                        request = request.map(|r| r.bearer_auth(access.secret()));
                        *tokens = Some(Tokens { access, refresh });
                        return Ok((has_refresh, request));
                    }
                    Err(e) => {
                        // Check to see if a reauthorization should occur
                        if let Some(handler) = &self.reauth
                            && !handler(Some(&e))
                        {
                            return Err(e.into());
                        }

                        // Fall back to device authorization
                    }
                }
            } else if let Some(handler) = &self.reauth {
                // Check to see if a reauthorization should occur
                if !handler(None) {
                    return Err(Error::NoRefreshToken);
                }
            }

            // Fall back to device authorization
        }

        // Initial request or couldn't refresh
        let (access, refresh) = oauth::authorize_device(&self.config, &self.prompt).await?;
        let has_refresh = refresh.is_some();
        request = request.map(|r| r.bearer_auth(access.secret()));
        *tokens = Some(Tokens { access, refresh });
        Ok((has_refresh, request))
    }
}

impl Authorizer for OAuthAuthorizer {
    fn authorize<'a>(
        &'a self,
        initial: bool,
        request: RequestBuilder,
    ) -> BoxFuture<'a, Result<RequestBuilder, Error>> {
        async move {
            // SAFETY: the given request is always returned upon success
            Ok(self
                .perform_authorization(initial, Some(request))
                .await?
                .1
                .unwrap())
        }
        .boxed()
    }

    fn reauthorizes(&self) -> bool {
        true
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::assert_matches;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use mockito::Mock;
    use mockito::Server;
    use mockito::ServerGuard;
    use pretty_assertions::assert_eq;
    use reqwest::Client;
    use reqwest::Method;
    use reqwest::StatusCode;

    use super::*;
    use crate::oauth::Config;

    pub struct OAuthTestServer {
        pub server: ServerGuard,
        pub device_endpoint: Mock,
        pub token_endpoint: Mock,
        pub config: Config,
    }

    impl OAuthTestServer {
        pub async fn new(with_refresh_token: bool) -> Self {
            let mut server = Server::new_async().await;

            let url = server.url();
            let device_endpoint = server.mock("POST", "/oauth/device")
                .match_body("client_id=12345")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{ "device_code": "54321", "user_code": "ABCD-EFGH", "verification_uri": "https://example.com", "expires_in": 100, "interval": 1 }"#)
                .create();

            let token_endpoint = server
                .mock("POST", "/oauth/token")
                .match_body(
                    "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&\
                     device_code=54321&client_id=12345",
                )
                .with_status(200)
                .with_body(if with_refresh_token {
                    r#"{ "access_token": "ABC", "refresh_token": "XYZ", "token_type": "Bearer" }"#
                } else {
                    r#"{ "access_token": "ABC", "token_type": "Bearer" }"#
                })
                .create();

            Self {
                server,
                device_endpoint,
                token_endpoint,
                config: Config {
                    client_id: "12345".into(),
                    client_secret: None,
                    authorization: format!("{url}/oauth/device").parse().unwrap(),
                    token: format!("{url}/oauth/token").parse().unwrap(),
                    scopes: Vec::new(),
                },
            }
        }

        pub fn assert(&self) {
            self.device_endpoint.assert();
            self.token_endpoint.assert();
        }
    }

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
        let request = authorizer.authorize(true, request).await.unwrap();
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
        let request = authorizer.authorize(true, request).await.unwrap();
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
        let request = authorizer.authorize(true, request).await.unwrap();
        request.send().await.unwrap();

        endpoint.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer() {
        let mut oauth = OAuthTestServer::new(true).await;

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        });

        assert!(authorizer.reauthorizes());

        let url = oauth.server.url();
        let endpoint = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer ABC")
            .with_status(200)
            .expect(2)
            .create();

        let client = Client::new();
        let request = client.request(Method::GET, url.to_string());
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        // Attempt another request (should not hit OAuth endpoints)
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        oauth.assert();
        endpoint.assert();
    }

    #[tokio::test]
    async fn test_oauth_initialization() {
        let oauth = OAuthTestServer::new(true).await;

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        });

        assert!(authorizer.reauthorizes());
        assert!(authorizer.initialize().await.unwrap());
        oauth.assert();

        let tokens = authorizer.tokens.lock().await;
        let tokens = tokens.as_ref().unwrap();

        assert_eq!(tokens.access.secret(), "ABC");
        assert_eq!(tokens.refresh.as_ref().unwrap().secret(), "XYZ");
    }

    #[tokio::test]
    async fn test_oauth_authorizer_refresh() {
        let mut oauth = OAuthTestServer::new(true).await;

        let refresh_endpoint = oauth
            .server
            .mock("POST", "/oauth/token")
            .match_body("grant_type=refresh_token&refresh_token=XYZ&client_id=12345")
            .with_status(200)
            .with_body(
                r#"{ "access_token": "DEF", "refresh_token": "123", "token_type": "Bearer" }"#,
            )
            .create();

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        });

        assert!(authorizer.reauthorizes());

        let url = oauth.server.url();
        let unauthorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer ABC")
            .with_status(401)
            .create();
        let authorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer DEF")
            .with_status(200)
            .expect(2)
            .create();

        let client = Client::new();
        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(
            request.send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(false, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        // Perform the request again; this should not hit OAuth endpoints
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        oauth.assert();
        refresh_endpoint.assert();
        unauthorized.assert();
        authorized.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer_refresh_failed() {
        let mut oauth = OAuthTestServer::new(true).await;

        let refresh_endpoint = oauth
            .server
            .mock("POST", "/oauth/token")
            .match_body("grant_type=refresh_token&refresh_token=XYZ&client_id=12345")
            .with_status(400)
            .with_body(
                r#"{ "error": "token_expired", "error_description": "the refresh token has expired" }"#,
            )
            .create();

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        });

        assert!(authorizer.reauthorizes());

        let url = oauth.server.url();
        let unauthorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer ABC")
            .with_status(401)
            .create();
        let authorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer DEF")
            .with_status(200)
            .create();

        let client = Client::new();
        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(
            request.send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        // The device will be reauthorized, so replace the token endpoint with
        // one that returns a new access token
        oauth.device_endpoint = oauth.device_endpoint.expect(2);
        oauth.token_endpoint.assert();
        oauth.token_endpoint.remove();
        oauth.token_endpoint = oauth
            .server
            .mock("POST", "/oauth/token")
            .match_body(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&\
                 device_code=54321&client_id=12345",
            )
            .with_status(200)
            .with_body(
                r#"{ "access_token": "DEF", "refresh_token": "123", "token_type": "Bearer" }"#,
            )
            .create();

        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(false, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        oauth.assert();
        refresh_endpoint.assert();
        unauthorized.assert();
        authorized.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer_no_refresh_token() {
        let mut oauth = OAuthTestServer::new(false).await;

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        });

        assert!(authorizer.reauthorizes());

        let url = oauth.server.url();
        let unauthorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer ABC")
            .with_status(401)
            .create();
        let authorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer DEF")
            .with_status(200)
            .create();

        let client = Client::new();
        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(
            request.send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        // The device will be reauthorized, so replace the token endpoint with
        // one that returns a new access token
        oauth.device_endpoint = oauth.device_endpoint.expect(2);
        oauth.token_endpoint.assert();
        oauth.token_endpoint.remove();
        oauth.token_endpoint = oauth
            .server
            .mock("POST", "/oauth/token")
            .match_body(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&\
                 device_code=54321&client_id=12345",
            )
            .with_status(200)
            .with_body(
                r#"{ "access_token": "DEF", "refresh_token": "123", "token_type": "Bearer" }"#,
            )
            .create();

        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(false, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        oauth.assert();
        unauthorized.assert();
        authorized.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer_reauth_denied() {
        let mut oauth = OAuthTestServer::new(false).await;

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .on_reauthorization(|e| {
            assert!(e.is_none());
            false
        });

        assert!(authorizer.reauthorizes());

        let url = oauth.server.url();
        let unauthorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer ABC")
            .with_status(401)
            .create();

        let client = Client::new();
        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(
            request.send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let request = client.request(Method::GET, url);
        assert_eq!(
            authorizer
                .authorize(false, request)
                .await
                .unwrap_err()
                .to_string(),
            "the client did not receive a refresh token and could not automatically refresh the \
             access token"
        );

        oauth.assert();
        unauthorized.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer_reauth_denied_on_error() {
        let mut oauth = OAuthTestServer::new(true).await;

        let refresh_endpoint = oauth
            .server
            .mock("POST", "/oauth/token")
            .match_body("grant_type=refresh_token&refresh_token=XYZ&client_id=12345")
            .with_status(400)
            .with_body(
                r#"{ "error": "token_expired", "error_description": "the refresh token has expired" }"#,
            )
            .create();

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .on_reauthorization(|e| {
            let e = e.unwrap();
            assert_matches!(e, oauth::Error::Basic(e) if e.error.to_string().contains("token_expired: the refresh token has expired"), "unexpected error: {e}");
            false
        });

        assert!(authorizer.reauthorizes());

        let url = oauth.server.url();
        let unauthorized = oauth
            .server
            .mock("GET", "/")
            .match_header("authorization", "Bearer ABC")
            .with_status(401)
            .create();

        let client = Client::new();
        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(
            request.send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let request = client.request(Method::GET, url);

        let error = authorizer.authorize(false, request).await.unwrap_err();

        assert_matches!(error, Error::Oauth(oauth::Error::Basic(e)) if e.error.to_string().contains("token_expired: the refresh token has expired"), "unexpected error: {error}");

        oauth.assert();
        unauthorized.assert();
        refresh_endpoint.assert();
    }
}
