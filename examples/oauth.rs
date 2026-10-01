//! Lists all tasks known about within an execution service using OAuth for
//! authentication.
//!
//! You can run this with the following command:
//!
//! ```bash
//! export OAUTH_CLIENT_ID="<CLIENT>"
//! export OAUTH_CLIENT_SECRET="<SECRET>" # this is optional
//! export OAUTH_AUDIENCE="<AUDIENCE>" # this is optional
//! export OAUTH_AUTHORIZATION_URI="<AUTHORIZATION_URI>"
//! export OAUTH_TOKEN_URI="<TOKEN_URI>"
//! export OAUTH_SCOPES="<SCOPE1>;<SCOPE2>;..."
//! export RUST_LOG="tes=debug"
//!
//! cargo run --release --features=oauth,serde --example oauth <URL>
//! ```

use std::env;

use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;
use tes::auth::oauth::Config;
use tes::auth::oauth::OAuthAuthorizer;
use tes::v1::client;
use tes::v1::client::Client;
use tes::v1::client::strategy::ExponentialFactorBackoff;
use tes::v1::client::strategy::MaxInterval;
use tes::v1::types::requests::ListTasksParams;
use tes::v1::types::requests::View;
use tracing_subscriber::EnvFilter;

/// The environment variable for an OAuth client identifier.
const OAUTH_CLIENT_ID: &str = "OAUTH_CLIENT_ID";

/// The environment variable for an OAuth client secret.
const OAUTH_CLIENT_SECRET: &str = "OAUTH_CLIENT_SECRET";

/// The environment variable for an OAuth audience.
const OAUTH_AUDIENCE: &str = "OAUTH_AUDIENCE";

/// The environment variable for an OAuth authorization URI.
const OAUTH_AUTHORIZATION_URI: &str = "OAUTH_AUTHORIZATION_URI";

/// The environment variable for an OAuth token URI.
const OAUTH_TOKEN_URI: &str = "OAUTH_TOKEN_URI";

/// The environment variable for OAuth scopes (semicolon delimited).
const OAUTH_SCOPES: &str = "OAUTH_SCOPES";

/// Lists all tasks on the server.
async fn list_all_tasks(client: &Client) -> Result<()> {
    let mut last_token = None;

    loop {
        let retries = ExponentialFactorBackoff::from_millis(1000, 2.0)
            .max_interval(10000)
            .take(3);

        let response = client
            .list_tasks(
                Some(&ListTasksParams {
                    view: Some(View::Full),
                    page_token: last_token,
                    ..Default::default()
                }),
                retries,
            )
            .await
            .into_diagnostic()
            .context("listing tasks")?;

        println!("{response:#?}");

        last_token = response.next_page_token;
        if last_token.is_none() {
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let url = std::env::args()
        .nth(1)
        .context("URL argument is required")?;

    let client_id = env::var(OAUTH_CLIENT_ID)
        .into_diagnostic()
        .with_context(|| format!("the `{OAUTH_CLIENT_ID}` environment variable is required"))?;

    let authorization = env::var(OAUTH_AUTHORIZATION_URI)
        .into_diagnostic()
        .with_context(|| {
            format!("the `{OAUTH_AUTHORIZATION_URI}` environment variable is required")
        })?;

    let token = env::var(OAUTH_TOKEN_URI)
        .into_diagnostic()
        .with_context(|| format!("the `{OAUTH_TOKEN_URI}` environment variable is required"))?;

    let scopes = env::var(OAUTH_SCOPES)
        .unwrap_or_default()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();

    let config = Config {
        client_id,
        client_secret: env::var(OAUTH_CLIENT_SECRET).ok(),
        audience: env::var(OAUTH_AUDIENCE).ok(),
        authorization: authorization
            .parse()
            .into_diagnostic()
            .with_context(|| format!("invalid authorization URL `{authorization}`"))?,
        token: token
            .parse()
            .into_diagnostic()
            .with_context(|| format!("invalid token URL `{token}`"))?,
        scopes,
    };

    let authorizer = OAuthAuthorizer::new(config, |response| {
        println!(
            "authorization is required: visit {url} and enter code `{code}`",
            url = response.verification_uri(),
            code = response.user_code().secret()
        );
    });

    let client = client::Builder::default()
        .url_from_string(url)
        .into_diagnostic()
        .context("URL could not be parsed")?
        .authorizer(authorizer)
        .try_build()
        .into_diagnostic()
        .context("failed to build TES client")?;

    list_all_tasks(&client).await?;

    Ok(())
}
