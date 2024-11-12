<p align="center">
  <h1 align="center">
    <code>tes</code>
  </h1>

  <div align="center" style="padding-bottom: 5px">
    <a href="https://github.com/stjude-rust-labs/tes/actions/workflows/CI.yml" target="_blank">
      <img alt="CI: Status" src="https://github.com/stjude-rust-labs/tes/actions/workflows/CI.yml/badge.svg" />
    </a>
    <a href="https://crates.io/crates/tes" target="_blank">
      <img alt="crates.io version" src="https://img.shields.io/crates/v/tes">
    </a>
    <img alt="crates.io downloads" src="https://img.shields.io/crates/d/tes">
    <a href="https://github.com/stjude-rust-labs/tes/blob/main/LICENSE-APACHE" target="_blank">
      <img alt="License: Apache 2.0" src="https://img.shields.io/badge/license-Apache 2.0-blue.svg" />
    </a>
    <a href="https://github.com/stjude-rust-labs/tes/blob/main/LICENSE-MIT" target="_blank">
      <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg" />
    </a>
  </div>

  <div align="center" style="padding-bottom: 5px">
    A crate for working with the Task Execution Service (TES) specification.
  </div>

  <p align="center">
    <a href="https://docs.rs/tes"><strong>Explore the docs »</strong></a> ·
    <a href="https://github.com/ga4gh/task-execution-schemas"><strong>Learn about TES »</strong></a>
  </p>
</p>

## 📚 Getting Started

The `tes` crate contains types (via the `types` feature) and a simple client
(via the `client` feature) for working with the [Task Execution Service
(TES)][tes-homepage] specification. Briefly, TES is a specification developed to
uniformly submit units of execution ("tasks") to multiple compute environment
through a single HTTP interface. It is of interest mostly when developing
clients or servers that participate in the large-scale submission or monitoring
of jobs.

To utilize `tes` in your crates, simply add it to your project.

```bash
# If you want to use the types.
cargo add tes

# If you also want to use the provided client.
cargo add tes --features client
```

After this, you can access the library using the `tes` module in your Rust code.
You can [take a look at the
examples](https://github.com/stjude-rust-labs/tes/tree/main/examples) for
inspiration, but a simple example could look like this.

```rust
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

```

### Minimum Supported Rust Version

The minimum supported Rust version is currently `1.80.0`.

There is a CI job that verifies the declared minimum supported version.

If a contributor submits a PR that uses a feature from a newer version of Rust,
the contributor is responsible for updating the minimum supported version in
the `Cargo.toml`.

Contributors may update the minimum supported version as-needed to the latest
stable release of Rust.

To facilitate the discovery of what the minimum supported version should be,
install the `cargo-msrv` tool:

```bash
cargo install cargo-msrv
```

And run the following command:

```bash
cargo msrv --min 1.80.0
```

If the reported version is newer than the crate's current minimum supported
version, an update is required.

## 🖥️ Development

To bootstrap a development environment, please use the following commands.

```bash
# Clone the repository
git clone git@github.com:stjude-rust-labs/tes.git
cd tes

# Build the crate in release mode
cargo build --release

# List out the examples
cargo run --release --example
```

## 🚧️ Tests

Before submitting any pull requests, please make sure the code passes the
following checks (from the root directory).

```bash
# Run the project's tests.
cargo test --all-features

# Run the tests for the examples.
cargo test --examples --all-features

# Ensure the project doesn't have any linting warnings.
cargo clippy --all-features

# Ensure the project passes `cargo fmt`.
# Currently this requires nightly Rust
cargo +nightly fmt --check

# Ensure the docs build.
cargo doc
```

## 🤝 Contributing

Contributions, issues and feature requests are welcome! Feel free to check
[issues page](https://github.com/stjude-rust-labs/tes/issues).

## 📝 License

This project is licensed as either [Apache 2.0][license-apache] or
[MIT][license-mit] at your discretion. Additionally, please see [the
disclaimer](https://github.com/stjude-rust-labs#disclaimer) that applies to all
crates and command line tools made available by St. Jude Rust Labs.

Copyright © 2024-Present [St. Jude Children's Research Hospital](https://github.com/stjude).

[tes-homepage]: https://www.ga4gh.org/product/task-execution-service-tes/
[license-apache]: https://github.com/stjude-rust-labs/tes/blob/main/LICENSE-APACHE
[license-mit]: https://github.com/stjude-rust-labs/tes/blob/main/LICENSE-MIT
