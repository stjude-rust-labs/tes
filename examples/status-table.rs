//! Lists the status of each task as a summary.
//!
//! You can run this with the following command:
//!
//! ```bash
//! export USER="<USER>"
//! export PASSWORD="<PASSWORD>"
//! export RUST_LOG="tes=debug"
//!
//! cargo run --release --features=client,serde --example status-table <URL>
//! ```

use std::collections::HashMap;

use base64::prelude::*;
use miette::Context as _;
use miette::IntoDiagnostic;
use miette::Result;
use miette::bail;
use tes::v1::client::Client;
use tes::v1::types::requests::ListTasksParams;
use tes::v1::types::requests::View;
use tes::v1::types::task::State;
use tokio_retry2::strategy::ExponentialFactorBackoff;
use tokio_retry2::strategy::MaxInterval as _;
use tracing_subscriber::EnvFilter;

/// The environment variable for a basic auth username.
const USER_ENV: &str = "USER";

/// The environment variable for a basic auth password.
const PASSWORD_ENV: &str = "PASSWORD";

/// A displayable version of a TES state.
#[derive(Eq, Hash, PartialEq)]
struct DisplayableState(State);

impl DisplayableState {
    /// Gets the associated order group for a particular state.
    fn ord_group(&self) -> usize {
        match self.0 {
            State::Unknown => 0,
            State::Queued | State::Initializing => 1,
            State::Running => 2,
            State::Paused => 3,
            State::Complete => 4,
            State::ExecutorError | State::SystemError => 5,
            State::Canceled | State::Canceling => 6,
            State::Preempted => 7,
        }
    }
}

impl std::fmt::Display for DisplayableState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            State::Unknown => write!(f, "UNKNOWN"),
            State::Queued => write!(f, "QUEUED"),
            State::Initializing => write!(f, "INITIALIZING"),
            State::Running => write!(f, "RUNNING"),
            State::Paused => write!(f, "PAUSED"),
            State::Complete => write!(f, "COMPLETE"),
            State::ExecutorError => write!(f, "EXECUTOR_ERROR"),
            State::SystemError => write!(f, "SYSTEM_ERROR"),
            State::Canceled => write!(f, "CANCELED"),
            State::Canceling => write!(f, "CANCELING"),
            State::Preempted => write!(f, "PREEMPTED"),
        }
    }
}

impl std::cmp::Ord for DisplayableState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ord_group().cmp(&other.ord_group())
    }
}

impl std::cmp::PartialOrd for DisplayableState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Prints a status table for the tasks in the server.
async fn print_status_table(client: &Client) -> Result<()> {
    let mut last_token = None;

    let mut states = HashMap::<Option<DisplayableState>, usize>::new();

    loop {
        let retries = ExponentialFactorBackoff::from_millis(1000, 2.0)
            .max_interval(10000)
            .take(3);

        let response = client
            .list_tasks(
                Some(&ListTasksParams {
                    view: Some(View::Minimal),
                    page_token: last_token,
                    ..Default::default()
                }),
                retries,
            )
            .await
            .into_diagnostic()
            .context("listing tasks")?;

        for state in response
            .tasks
            .into_iter()
            .map(|task| task.into_minimal().unwrap().state.map(DisplayableState))
        {
            *states.entry(state).or_default() += 1;
        }

        last_token = response.next_page_token;
        if last_token.is_none() {
            break;
        }
    }

    let mut states = states.into_iter().collect::<Vec<_>>();
    states.sort();

    println!("+--------------------+-----------+");
    println!("| State              | Count     | ");
    println!("+--------------------+-----------+");
    for (state, count) in states {
        println!(
            "| {state: <18} | {count: >9} |",
            state = state
                .map(|state| state.to_string())
                .unwrap_or(String::from("<unknown>")),
            count = count
        );
    }
    println!("+--------------------+-----------+");

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

    let mut builder = Client::builder()
        .url_from_string(url)
        .into_diagnostic()
        .context("URL could not be parsed")?;

    let username = std::env::var(USER_ENV).ok();
    let password = std::env::var(PASSWORD_ENV).ok();

    if (username.is_some() && password.is_none()) || (username.is_none() && password.is_some()) {
        bail!("${USER_ENV} and ${PASSWORD_ENV} must both be set to use basic auth");
    }

    if let Some(username) = username {
        let credentials = format!("{}:{}", username, password.unwrap());
        let encoded = BASE64_STANDARD.encode(credentials);
        builder = builder.insert_header("Authorization", format!("Basic {encoded}"));
    }

    let client = builder
        .try_build()
        .into_diagnostic()
        .context("failed to build TES client")?;
    print_status_table(&client).await?;

    Ok(())
}
