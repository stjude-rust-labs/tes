//! Submits an example task to an execution service.
//!
//! You can run this with the following command:
//!
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//! export RUST_LOG="tes=debug"
//!
//! cargo run --release --features=client,serde --example task-submit <URL>
//! ```

use base64::prelude::*;
use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;
use tes::v1::client;
use tes::v1::client::strategy::ExponentialFactorBackoff;
use tes::v1::client::strategy::MaxInterval;
use tes::v1::types::requests::Task;
use tes::v1::types::task::Executor;
use tes::v1::types::task::Resources;
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
        builder = builder.insert_header("Authorization", format!("Basic {encoded}"));
    }

    let client = builder.try_build().expect("could not build client");

    let task = Task {
        name: Some(String::from("my-task")),
        description: Some(String::from("A description.")),
        resources: Some(Resources {
            cpu_cores: Some(4),
            preemptible: Some(true),
            ..Default::default()
        }),
        executors: vec![Executor {
            image: String::from("ubuntu:latest"),
            command: vec![
                String::from("/bin/bash"),
                String::from("-c"),
                String::from("echo 'hello, world!'"),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    let retries = ExponentialFactorBackoff::from_millis(1000, 2.0)
        .max_interval(10000)
        .take(3);

    println!(
        "{:#?}",
        client
            .create_task(&task, retries)
            .await
            .into_diagnostic()
            .context("submitting a task")?
    );

    Ok(())
}
