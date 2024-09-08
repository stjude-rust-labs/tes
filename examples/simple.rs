//! A simple example of using the client.
//!
//! You can run this with the following command:
//!
//! `cargo run --release --features=client --example simple <URL>`

use tes::v1::client;

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("url to be present");

    let client = client::Builder::default()
        .url_from_string(url)
        .expect("url could not be parsed")
        .try_build()
        .expect("could not build client");

    println!(
        "{:#?}",
        client
            .service_info()
            .await
            .expect("getting service information failed")
    );
}
