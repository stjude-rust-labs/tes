//! Lists all tasks known about within an execution service.
//!
//! You can run this with the following command:
//!
//! `TOKEN=<TOKEN> RUST_LOG=tes=debug cargo run --release --features=client
//! --example task-list-all <URL>`

use anyhow::Context;
use anyhow::Result;
use tes::v1::client;
use tes::v1::client::tasks::View;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let url = std::env::args().nth(1).expect("url to be present");

    let mut builder = client::Builder::default()
        .url_from_string(url)
        .expect("url could not be parsed");

    if let Ok(token) = std::env::var("TOKEN") {
        builder = builder.insert_header("Authorization", format!("Basic {}", token));
    }

    let client = builder.try_build().expect("could not build client");

    println!(
        "{:#?}",
        client
            .list_all_tasks(View::Full)
            .await
            .context("listing all tasks")?
    );

    Ok(())
}
