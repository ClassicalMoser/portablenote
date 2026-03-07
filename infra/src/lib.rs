//! # portablenote-infra
//!
//! Infrastructure adapters that implement the port traits defined in
//! `portablenote-core::application::ports`. This crate contains all I/O —
//! the core crate remains pure.
//!
//! ## Modules
//!
//! - **`fs`** — Filesystem adapters. Each vault artifact maps to files on disk:
//!   blocks as `.md` files, the graph as `block-graph.json`, documents as
//!   individual JSON files, and the name index as `names.json`.
//! - **`clock`** — System clock implementing the `Clock` port.

pub mod clock;
pub mod fs;

pub use clock::SystemClock;
