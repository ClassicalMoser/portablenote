//! Application layer — use cases orchestrated against port traits.
//!
//! This layer sits between the domain and infrastructure. Use cases accept
//! port trait references (`&dyn BlockStore`, etc.), call domain functions,
//! and return either direct events (single-store mutations) or result structs
//! (multi-store mutations) that the caller must apply atomically.
//!
//! - **`ports`** — Trait definitions for storage backends (the hexagonal boundary)
//! - **`results`** — Changeset structs for multi-store use cases
//! - **`use_cases`** — One module per spec command, each exposing an `execute` function

pub mod ports;
pub mod results;
pub mod use_cases;
