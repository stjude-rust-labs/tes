# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Adds `v1::types::responses::task::View` so it can be accepted as a query
  parameter in downstream crates.

### Revised

- Promotes the `v1::types::responses::service` module to public.
- Gates serialization/deserialization behind the `serde` feature.
- Makes most structs `Clone`.
- Converts `ServiceInfo` to use a builder for construction.
- Changes the `v1::types::responses::service` module to
  `v1::types::responses::service_info`.
- Makes `v1::types::task::State` `Copy`.
- Adds the `ord` feature for all types.

### Fixed

- Removed errant `#[serde(untagged)]` for `v1::types::responses::task::View`.
- Fixes multiple fields that should not be serialized if `None`
  ([#4](https://github.com/stjude-rust-labs/tes/pull/4)).
- Corrects the `outputs` key on a `TaskLog` to be non-optional
  ([#4](https://github.com/stjude-rust-labs/tes/pull/4)).

## 0.2.0 - 08-08-2024

### Added

- Added initial version of the crate.

## 0.1.0

> A note on v0.1.0: this version was accidentally released when reserving the
> `tes` package name on [crates.io](https://crates.io/crates/tes). As such,
> v0.1.0 is technically an empty release: it was yanked from crates.io, and
> v0.2.0 is the first _real_ version of the crate.
