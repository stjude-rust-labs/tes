//! An implementation of OAuth device authorization used by TES clients.

use oauth2::AccessToken;
use oauth2::ClientId;
use oauth2::ClientSecret;
use oauth2::DeviceAuthorizationUrl;
use oauth2::RefreshToken;
use oauth2::Scope;
use oauth2::StandardDeviceAuthorizationResponse;
use oauth2::TokenResponse;
use oauth2::TokenUrl;
use oauth2::basic::BasicClient;
use oauth2::basic::BasicTokenType;
use oauth2::reqwest;
use tracing::debug;
use url::Url;

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
    oauth2::HttpClientError<oauth2::reqwest::Error>,
    oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
>;

/// The error type for device code OAuth errors.
pub type DeviceCodeResponseError = oauth2::RequestTokenError<
    oauth2::HttpClientError<oauth2::reqwest::Error>,
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

/// A basic error from an OAuth request.
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

/// Authorizes a device via the OAuth device authorization flow.
///
/// Upon success, returns the access token and optional refresh token.
pub(crate) async fn authorize_device<P>(
    config: &Config,
    prompt: &P,
) -> Result<(AccessToken, Option<RefreshToken>), Error>
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

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    debug!(
        "requesting OAuth device authorization from `{url}`",
        url = config.authorization
    );

    let auth_response: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()
        .add_scopes(config.scopes.iter().cloned().map(Scope::new))
        .request_async(&http_client)
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
        .request_async(&http_client, tokio::time::sleep, None)
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

    // Only bearer tokens are supported in this implementation
    if *response.token_type() != BasicTokenType::Bearer {
        return Err(Error::UnsupportedAccessToken);
    }

    Ok((
        response.access_token().clone(),
        response.refresh_token().cloned(),
    ))
}

/// Performs a refresh of an access token provided a refresh token.
///
/// Upon success, returns the new access token and new refresh token.
pub(crate) async fn refresh_token(
    config: &Config,
    token: &RefreshToken,
) -> Result<(AccessToken, Option<RefreshToken>), Error> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

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
        .request_async(&http_client)
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

    Ok((
        response.access_token().clone(),
        response.refresh_token().cloned(),
    ))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use mockito::Server;
    use pretty_assertions::assert_eq;

    use super::*;

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

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let (access, refresh) = authorize_device(&config, &|response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .await
        .unwrap();

        assert!(invoked.load(Ordering::SeqCst));
        assert_eq!(access.secret(), "ABC");
        assert_eq!(refresh.as_ref().map(|t| t.secret().as_str()), Some("XYZ"));

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

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let (access, refresh) = authorize_device(&config, &|response| {
            invoked_clone.store(true, Ordering::SeqCst);
            assert_eq!(response.device_code().secret(), "54321");
            assert_eq!(response.user_code().secret(), "ABCD-EFGH");
            assert_eq!(response.expires_in(), Duration::from_secs(100));
            assert_eq!(response.verification_uri().as_str(), "https://example.com");
        })
        .await
        .unwrap();

        assert!(invoked.load(Ordering::SeqCst));
        assert_eq!(access.secret(), "ABC");
        assert_eq!(refresh.as_ref().map(|t| t.secret().as_str()), Some("XYZ"));

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

        let (access, refresh) = refresh_token(&config, &RefreshToken::new("XYZ".into()))
            .await
            .unwrap();

        assert_eq!(access.secret(), "ABC");
        assert_eq!(refresh.as_ref().map(|t| t.secret().as_str()), Some("123"));

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

        let (access, refresh) = refresh_token(&config, &RefreshToken::new("XYZ".into()))
            .await
            .unwrap();

        assert_eq!(access.secret(), "ABC");
        assert_eq!(refresh.as_ref().map(|t| t.secret().as_str()), Some("123"));

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
            authorize_device(&config, &|_| {})
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

        let error = authorize_device(&config, &|_| {}).await.unwrap_err();

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

        let error = authorize_device(&config, &|_| {}).await.unwrap_err();

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

        let error = refresh_token(&config, &RefreshToken::new("XYZ".into()))
            .await
            .unwrap_err();

        assert_matches!(error, Error::Basic(e) if e.error.to_string().contains("invalid_request: wrong!"), "unexpected error: {error}");

        token_endpoint.assert();
    }
}
