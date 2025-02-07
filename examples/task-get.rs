//! Gets a particular task within an execution service.
//!
//! You can run this with the following command:
//!
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//! export RUST_LOG="tes=debug"
//!
//! cargo run --release --features=client,serde --example task-submit <URL> <ID>
//! ```

use base64::prelude::*;
use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;
use tes::v1::client;
use tes::v1::client::tasks::View;
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

    let url = std::env::args().nth(1).expect("url to be present");
    let id = std::env::args().nth(2).expect("task id to be present");

    let mut builder = client::Builder::default()
        .url_from_string(url)
        .expect("url could not be parsed");

    let username = std::env::var(USER_ENV).ok();
    let password = std::env::var(PASSWORD_ENV).ok();

    if (username.is_some() && password.is_none()) || (username.is_none() && password.is_some()) {
        panic!("${USER_ENV} and ${PASSWORD_ENV} must both be set to use basic auth");
    }

    if let Some(username) = username {
        let credentials = format!("{}:{}", username, password.unwrap());
        let encoded = BASE64_STANDARD.encode(credentials);
        builder = builder.insert_header("Authorization", format!("Basic {}", encoded));
    }

    let client = builder.try_build().expect("could not build client");

    println!(
        "{:#?}",
        client
            .get_task(id, View::Full)
            .await
            .into_diagnostic()
            .context("getting a task")?
    );

    Ok(())
}
