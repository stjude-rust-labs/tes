//! A simple example of using the client.
//!
//! You can run this with the following command:
//!
//! `cargo run --release --features=client,serde --example simple <URL>`

use tes::v1::client;
use tes::v1::client::strategy::ExponentialFactorBackoff;
use tes::v1::client::strategy::MaxInterval;

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("url to be present");

    let client = client::Builder::default()
        .url_from_string(url)
        .expect("url could not be parsed")
        .try_build()
        .expect("could not build client");

    let retries = ExponentialFactorBackoff::from_millis(1000, 2.0)
        .max_interval(10000)
        .take(3);

    println!(
        "{:#?}",
        client
            .service_info(retries)
            .await
            .expect("getting service information failed")
    );
}
