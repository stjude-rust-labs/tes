//! An implementation of OAuth device authorization used by TES clients.

use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use futures::future::BoxFuture;
use oauth2::AccessToken;
use oauth2::ClientId;
use oauth2::ClientSecret;
use oauth2::DeviceAuthorizationUrl;
use oauth2::HttpClientError;
use oauth2::RefreshToken;
use oauth2::Scope;
use oauth2::StandardDeviceAuthorizationResponse;
use oauth2::TokenResponse;
use oauth2::TokenUrl;
use oauth2::basic::BasicClient;
use oauth2::basic::BasicTokenType;
use reqwest::Client;
use reqwest::ClientBuilder;
use reqwest::RequestBuilder;
use reqwest::redirect;
use tokio::sync::Mutex;
use tracing::debug;
use url::Url;

use crate::auth::Authorizer;

/// Helper function for sending an OAuth request through `reqwest`.
async fn send_request(
    client: Client,
    request: oauth2::HttpRequest,
) -> Result<oauth2::HttpResponse, oauth2::HttpClientError<reqwest::Error>> {
    let response = client
        .execute(reqwest::Request::try_from(request).map_err(Box::new)?)
        .await
        .map_err(Box::new)?;

    let mut builder = oauth2::http::response::Builder::new()
        .status(response.status())
        .version(response.version());

    for (name, value) in response.headers().iter() {
        builder = builder.header(name, value);
    }

    builder
        .body(response.bytes().await.map_err(Box::new)?.to_vec())
        .map_err(HttpClientError::Http)
}

/// Helper for extracting the tokens of a token response.
fn extract_tokens(
    response: &impl TokenResponse<TokenType = BasicTokenType>,
) -> Result<Tokens, Error> {
    // Only bearer tokens are supported in this implementation
    if *response.token_type() != BasicTokenType::Bearer {
        return Err(Error::UnsupportedAccessToken);
    }

    Ok(Tokens {
        access: response.access_token().clone(),
        refresh: response.refresh_token().cloned(),
    })
}

/// Represents configuration for OAuth.
#[derive(Clone)]
pub struct Config {
    /// The OAuth client identifier.
    pub client_id: String,
    /// The optional OAuth client secret.
    pub client_secret: Option<String>,
    /// The URL for OAuth authorization requests.
    pub authorization: Url,
    /// The URL for OAuth token requests.
    pub token: Url,
    /// The desired scopes for OAuth.
    pub scopes: Vec<String>,
}

/// The error type for basic OAuth errors.
pub type BasicResponseError = oauth2::RequestTokenError<
    oauth2::HttpClientError<reqwest::Error>,
    oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
>;

/// The error type for device code OAuth errors.
pub type DeviceCodeResponseError = oauth2::RequestTokenError<
    oauth2::HttpClientError<reqwest::Error>,
    oauth2::StandardErrorResponse<oauth2::DeviceCodeErrorResponseType>,
>;

/// A basic error from an OAuth request.
#[derive(Debug, thiserror::Error)]
#[error("error response from `{uri}`")]
pub struct BasicError {
    /// The URI of the request.
    pub uri: Url,
    /// The response error.
    #[source]
    pub error: BasicResponseError,
}

/// An error from an OAuth device access token request.
#[derive(Debug, thiserror::Error)]
#[error("error response from `{uri}`")]
pub struct DeviceCodeError {
    /// The URI of the request.
    pub uri: Url,
    /// The response error.
    #[source]
    pub error: DeviceCodeResponseError,
}

/// Represents an OAuth error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A reqwest error occurred.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// A basic error occurred during an OAuth request.
    #[error(transparent)]
    Basic(Box<BasicError>),
    /// An error occurred during the device code exchange.
    #[error(transparent)]
    DeviceCode(Box<DeviceCodeError>),
    /// The access token returned by the OAuth token endpoint is unsupported.
    #[error("OAuth access token is not supported: only bearer tokens are supported")]
    UnsupportedAccessToken,
}

/// Collection of OAuth-related tokens.
#[derive(Debug)]
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
type ReauthFn = dyn Fn(Option<&Error>) -> BoxFuture<'_, bool> + Send + Sync;

/// Implements an [`Authorizer`] for OAuth authorization.
pub struct OAuthAuthorizer {
    /// The OAuth configuration.
    config: Config,
    /// The client used for communicating with the OAuth service.
    client: Client,
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
    pub fn new<P>(config: Config, prompt: P) -> Self
    where
        P: Fn(&StandardDeviceAuthorizationResponse) + Send + Sync + 'static,
    {
        let client = ClientBuilder::new()
            .redirect(redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .expect("client should build");

        Self {
            config,
            client,
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
        E: Fn(Option<&Error>) -> BoxFuture<'_, bool> + Send + Sync + 'static,
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
    pub async fn initialize(&self) -> Result<bool, super::Error> {
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
    ) -> Result<(bool, Option<RequestBuilder>), super::Error> {
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
                match refresh_token(&self.config, &self.client, refresh).await {
                    Ok(new_tokens) => {
                        let has_refresh = new_tokens.refresh.is_some();
                        request = request.map(|r| r.bearer_auth(new_tokens.access.secret()));
                        *tokens = Some(new_tokens);
                        return Ok((has_refresh, request));
                    }
                    Err(e) => {
                        // Check to see if a reauthorization should occur
                        if let Some(handler) = &self.reauth
                            && !handler(Some(&e)).await
                        {
                            return Err(e.into());
                        }

                        // Fall back to device authorization
                    }
                }
            } else if let Some(handler) = &self.reauth {
                // Check to see if a reauthorization should occur
                if !handler(None).await {
                    return Err(super::Error::ReauthorizationDeclined);
                }
            }

            // Fall back to device authorization
        }

        // Initial request or couldn't refresh
        let new_tokens = authorize_device(&self.config, &self.client, &self.prompt).await?;
        let has_refresh = new_tokens.refresh.is_some();
        request = request.map(|r| r.bearer_auth(new_tokens.access.secret()));
        *tokens = Some(new_tokens);
        Ok((has_refresh, request))
    }
}

impl Authorizer for OAuthAuthorizer {
    fn authorize<'a>(
        &'a self,
        initial: bool,
        request: RequestBuilder,
    ) -> BoxFuture<'a, Result<RequestBuilder, super::Error>> {
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

/// Authorizes a device via the OAuth device authorization flow.
///
/// Upon success, returns the access token and optional refresh token.
async fn authorize_device<P>(
    config: &Config,
    http_client: &Client,
    prompt: &P,
) -> Result<Tokens, Error>
where
    P: Fn(&StandardDeviceAuthorizationResponse),
{
    let mut client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_device_authorization_url(DeviceAuthorizationUrl::from_url(
            config.authorization.clone(),
        ))
        .set_token_uri(TokenUrl::from_url(config.token.clone()));

    if let Some(secret) = &config.client_secret {
        client = client.set_client_secret(ClientSecret::new(secret.clone()));
    }

    debug!(
        "requesting OAuth device authorization from `{url}`",
        url = config.authorization
    );

    let auth_response: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()
        .add_scopes(config.scopes.iter().cloned().map(Scope::new))
        .request_async(&|request| send_request(http_client.clone(), request))
        .await
        .map_err(|error| {
            Error::Basic(
                BasicError {
                    uri: config.authorization.clone(),
                    error,
                }
                .into(),
            )
        })?;

    prompt(&auth_response);

    debug!(
        "requesting OAuth access token from `{url}`",
        url = config.token
    );

    let response = client
        .exchange_device_access_token(&auth_response)
        .request_async(
            &move |request| send_request(http_client.clone(), request),
            tokio::time::sleep,
            None,
        )
        .await
        .map_err(|error| {
            Error::DeviceCode(
                DeviceCodeError {
                    uri: config.token.clone(),
                    error,
                }
                .into(),
            )
        })?;

    extract_tokens(&response)
}

/// Performs a refresh of an access token provided a refresh token.
///
/// Upon success, returns the new access token and new refresh token.
async fn refresh_token(
    config: &Config,
    http_client: &Client,
    token: &RefreshToken,
) -> Result<Tokens, Error> {
    let mut client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_token_uri(TokenUrl::from_url(config.token.clone()));

    if let Some(secret) = &config.client_secret {
        client = client.set_client_secret(ClientSecret::new(secret.clone()));
    }

    debug!(
        "attempting to refresh OAuth access token from `{url}`",
        url = config.token
    );

    let response = client
        .exchange_refresh_token(token)
        .add_scopes(config.scopes.iter().cloned().map(Scope::new))
        .request_async(&move |request| send_request(http_client.clone(), request))
        .await
        .map_err(|error| {
            Error::Basic(
                BasicError {
                    uri: config.token.clone(),
                    error,
                }
                .into(),
            )
        })?;

    extract_tokens(&response)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::assert_matches;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
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
    async fn test_authorize_device() {
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
            .with_body(
                r#"{ "access_token": "ABC", "refresh_token": "XYZ", "token_type": "Bearer" }"#,
            )
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: None,
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let tokens = authorize_device(&config, &Client::new(), &|response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .await
        .unwrap();

        assert_eq!(invoked.load(Ordering::SeqCst), 1);
        assert_eq!(tokens.access.secret(), "ABC");
        assert_eq!(
            tokens.refresh.as_ref().map(|t| t.secret().as_str()),
            Some("XYZ")
        );

        device_endpoint.assert();
        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_authorize_device_with_secret() {
        let mut server = Server::new_async().await;

        let url = server.url();
        let device_endpoint = server.mock("POST", "/oauth/device")
            .match_header("authorization", "Basic MTIzNDU6c2VjcmV0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "device_code": "54321", "user_code": "ABCD-EFGH", "verification_uri": "https://example.com", "expires_in": 100, "interval": 1 }"#)
            .create();

        let token_endpoint = server
            .mock("POST", "/oauth/token")
            .match_header("authorization", "Basic MTIzNDU6c2VjcmV0")
            .match_body(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&\
                 device_code=54321",
            )
            .with_status(200)
            .with_body(
                r#"{ "access_token": "ABC", "refresh_token": "XYZ", "token_type": "Bearer" }"#,
            )
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: Some("secret".into()),
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let tokens = authorize_device(&config, &Client::new(), &|response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .await
        .unwrap();

        assert_eq!(invoked.load(Ordering::SeqCst), 1);
        assert_eq!(tokens.access.secret(), "ABC");
        assert_eq!(
            tokens.refresh.as_ref().map(|t| t.secret().as_str()),
            Some("XYZ")
        );

        device_endpoint.assert();
        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let mut server = Server::new_async().await;

        let url = server.url();

        let token_endpoint = server
            .mock("POST", "/oauth/token")
            .match_body("grant_type=refresh_token&refresh_token=XYZ&client_id=12345")
            .with_status(200)
            .with_body(
                r#"{ "access_token": "ABC", "refresh_token": "123", "token_type": "Bearer" }"#,
            )
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: None,
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let tokens = refresh_token(&config, &Client::new(), &RefreshToken::new("XYZ".into()))
            .await
            .unwrap();

        assert_eq!(tokens.access.secret(), "ABC");
        assert_eq!(
            tokens.refresh.as_ref().map(|t| t.secret().as_str()),
            Some("123")
        );

        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_refresh_token_with_secret() {
        let mut server = Server::new_async().await;

        let url = server.url();

        let token_endpoint = server
            .mock("POST", "/oauth/token")
            .match_header("authorization", "Basic MTIzNDU6c2VjcmV0")
            .match_body("grant_type=refresh_token&refresh_token=XYZ")
            .with_status(200)
            .with_body(
                r#"{ "access_token": "ABC", "refresh_token": "123", "token_type": "Bearer" }"#,
            )
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: Some("secret".into()),
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let tokens = refresh_token(&config, &Client::new(), &RefreshToken::new("XYZ".into()))
            .await
            .unwrap();

        assert_eq!(tokens.access.secret(), "ABC");
        assert_eq!(
            tokens.refresh.as_ref().map(|t| t.secret().as_str()),
            Some("123")
        );

        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_not_bearer_token() {
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
            .with_body(r#"{ "access_token": "ABC", "refresh_token": "XYZ", "token_type": "Foo" }"#)
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: None,
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        assert_eq!(
            authorize_device(&config, &Client::new(), &|_| {})
                .await
                .unwrap_err()
                .to_string(),
            "OAuth access token is not supported: only bearer tokens are supported"
        );

        device_endpoint.assert();
        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_authorize_device_with_device_endpoint_error() {
        let mut server = Server::new_async().await;

        let url = server.url();
        let device_endpoint = server
            .mock("POST", "/oauth/device")
            .match_body("client_id=12345")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "error": "invalid_request", "error_description": "wrong!" }"#)
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: None,
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let error = authorize_device(&config, &Client::new(), &|_| {})
            .await
            .unwrap_err();

        assert_matches!(error, Error::Basic(e) if e.error.to_string().contains("invalid_request: wrong!"), "unexpected error: {error}");
        device_endpoint.assert();
    }

    #[tokio::test]
    async fn test_authorize_device_with_token_endpoint_error() {
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
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "error": "invalid_request", "error_description": "wrong!" }"#)
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: None,
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let error = authorize_device(&config, &Client::new(), &|_| {})
            .await
            .unwrap_err();

        assert_matches!(error, Error::DeviceCode(e) if e.error.to_string().contains("invalid_request: wrong!"), "unexpected error: {error}");

        device_endpoint.assert();
        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_refresh_token_with_token_endpoint_error() {
        let mut server = Server::new_async().await;

        let url = server.url();
        let token_endpoint = server
            .mock("POST", "/oauth/token")
            .match_body("grant_type=refresh_token&refresh_token=XYZ&client_id=12345")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "error": "invalid_request", "error_description": "wrong!" }"#)
            .create();

        let config = Config {
            client_id: "12345".into(),
            client_secret: None,
            authorization: format!("{url}/oauth/device").parse().unwrap(),
            token: format!("{url}/oauth/token").parse().unwrap(),
            scopes: Vec::new(),
        };

        let error = refresh_token(&config, &Client::new(), &RefreshToken::new("XYZ".into()))
            .await
            .unwrap_err();

        assert_matches!(error, Error::Basic(e) if e.error.to_string().contains("invalid_request: wrong!"), "unexpected error: {error}");

        token_endpoint.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer() {
        let mut oauth = OAuthTestServer::new(true).await;

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
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

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        // Attempt another request (should not hit OAuth endpoints)
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        oauth.assert();
        endpoint.assert();
    }

    #[tokio::test]
    async fn test_oauth_initialization() {
        let oauth = OAuthTestServer::new(true).await;

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        });

        assert!(authorizer.reauthorizes());
        assert!(authorizer.initialize().await.unwrap());
        oauth.assert();

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

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

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
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

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        let request = client.request(Method::GET, url.clone());
        let request = authorizer.authorize(false, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        // Perform the request again; this should not hit OAuth endpoints
        let request = client.request(Method::GET, url);
        let request = authorizer.authorize(true, request).await.unwrap();
        assert_eq!(request.send().await.unwrap().status(), StatusCode::OK);

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

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

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
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

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

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

        assert_eq!(invoked.load(Ordering::SeqCst), 2);

        oauth.assert();
        refresh_endpoint.assert();
        unauthorized.assert();
        authorized.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer_no_refresh_token() {
        let mut oauth = OAuthTestServer::new(false).await;

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
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

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

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

        assert_eq!(invoked.load(Ordering::SeqCst), 2);

        oauth.assert();
        unauthorized.assert();
        authorized.assert();
    }

    #[tokio::test]
    async fn test_oauth_authorizer_reauth_denied() {
        let mut oauth = OAuthTestServer::new(false).await;

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .on_reauthorization(|e| {
            async move {
                assert!(e.is_none());
                false
            }
            .boxed()
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

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        let request = client.request(Method::GET, url);
        assert_eq!(
            authorizer
                .authorize(false, request)
                .await
                .unwrap_err()
                .to_string(),
            "reauthorization was declined because no refresh token was available"
        );

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

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

        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();
        let authorizer = OAuthAuthorizer::new(oauth.config.clone(), move |response| {
            invoked_clone.fetch_add(1, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .on_reauthorization(|e| async move {
            let e = e.unwrap();
            assert_matches!(e, Error::Basic(e) if e.error.to_string().contains("token_expired: the refresh token has expired"), "unexpected error: {e}");
            false
        }.boxed());

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

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        let request = client.request(Method::GET, url);
        let error = authorizer.authorize(false, request).await.unwrap_err();

        assert_matches!(
            error,
            crate::auth::Error::OAuth(Error::Basic(e)) if e.error.to_string().contains("token_expired: the refresh token has expired"), "unexpected error: {error}"
        );

        assert_eq!(invoked.load(Ordering::SeqCst), 1);

        oauth.assert();
        unauthorized.assert();
        refresh_endpoint.assert();
    }
}
