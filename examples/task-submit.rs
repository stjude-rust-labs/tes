//! Submits an example task to an execution service.
//!
//! You can run this with the following command:
//!
//! `TOKEN=<TOKEN> RUST_LOG=tes=debug cargo run --release --features=client
//! --example task-submit <URL>`

use anyhow::Context;
use anyhow::Result;
use tes::v1::client;
use tes::v1::types::Task;
use tes::v1::types::task::Executor;
use tes::v1::types::task::Resources;
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

    println!(
        "{:#?}",
        client
            .create_task(task)
            .await
            .context("submitting a task")?
    );

    Ok(())
}
