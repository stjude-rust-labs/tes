//! Lists all tasks known about within an execution service.
//!
//! You can run this with the following command:
//!
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//! export RUST_LOG="tes=debug"
//!
//! cargo run --release --features=client,serde --example task-list-all <URL>
//! ```

use base64::prelude::*;
use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;
use tes::v1::client::Client;
use tes::v1::types::requests::ListTasksParams;
use tes::v1::types::requests::View;
use tracing_subscriber::EnvFilter; // Import the Engine trait

/// The environment variable for a basic auth username.
const USER_ENV: &str = "USER";

/// The environment variable for a basic auth password.
const PASSWORD_ENV: &str = "PASSWORD";

/// Lists all tasks on the server.
async fn list_all_tasks(client: &Client) -> Result<()> {
    let mut last_token = None;

    loop {
        let response = client
            .list_tasks(Some(&ListTasksParams {
                view: View::Full,
                page_token: last_token,
                ..Default::default()
            }))
            .await
            .into_diagnostic()
            .context("listing tasks")?;

        println!("{:#?}", response);

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

    let url = std::env::args().nth(1).expect("url to be present");

    let mut builder = Client::builder()
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
    list_all_tasks(&client).await?;

    Ok(())
}
