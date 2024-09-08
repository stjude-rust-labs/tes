//! Facilities for working with the Task Execution Service specification.
//!
//! The Task Execution Service (TES) specification is an effort organized by the
//! Global Alliance for Genomics and Health (GA4GH) to define a common API
//! standard for describing and executing batched execution tasks. You can learn
//! more about the specification at the dedicated [website] or the [Swagger
//! Editor][swagger].
//!
//! At present, versions 1.x of the specification are supported.
//!
//! ## Features
//!
//! This crate provides the following features.
#![doc = include_str!("../docs/FEATURES.md")]
//! [website]: https://ga4gh.github.io/task-execution-schemas/
//! [swagger]:
//!     https://editor.swagger.io/?url=https://ga4gh.github.io/task-execution-schemas/openapi.yaml

pub mod v1;
