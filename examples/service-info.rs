//! Gets the descriptive information of the execution service.
//!
//! You can run this with the following command:
//!
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//! export RUST_LOG="tes=debug"
//!
//! cargo run --release --features=client,serde --example service-info <URL>
//! ```

use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;
use tes::auth::BasicAuthorizer;
use tes::v1::client;
use tes::v1::client::strategy::ExponentialFactorBackoff;
use tes::v1::client::strategy::MaxInterval;
use tracing_subscriber::EnvFilter;

/// The environment variable for a basic auth username.
const USER_ENV: &str = "USER";

/// The environment variable for a basic auth password.
const PASSWORD_ENV: &str = "PASSWORD";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let url = std::env::args()
        .nth(1)
        .context("URL argument is required")?;

    let username = std::env::var(USER_ENV).ok();
    let password = std::env::var(PASSWORD_ENV).ok();

    if username.is_none() && password.is_some() {
        panic!("${USER_ENV} and ${PASSWORD_ENV} must both be set to use basic auth");
    }

    let authorizer = username.map(|username| BasicAuthorizer::new(username, password));

    let client = client::Builder::default()
        .url_from_string(url)
        .into_diagnostic()
        .context("URL could not be parsed")?
        .maybe_authorizer(authorizer)
        .try_build()
        .into_diagnostic()
        .context("failed to build TES client")?;

    let retries = ExponentialFactorBackoff::from_millis(1000, 2.0)
        .max_interval(10000)
        .take(3);

    println!(
        "{:#?}",
        client
            .service_info(retries)
            .await
            .into_diagnostic()
            .context("getting the service information")?
    );

    Ok(())
}
