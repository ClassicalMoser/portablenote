//! Application layer — use cases orchestrated against port traits.
//!
//! This layer sits between the domain and infrastructure. Use cases accept
//! port trait references (`&dyn BlockStore`, etc.), call domain functions,
//! and return either direct events (single-store mutations) or result structs
//! (multi-store mutations) that the caller must apply atomically.
//!
//! - **`ports`** — Trait definitions and `VaultPorts` (injected port bag)
//! - **`results`** — Changeset structs for multi-store use cases
//! - **`runner`** — `UseCases` surface: `UseCases::new(ports)` at composition root
//! - **`use_cases`** — One module per spec command, each exposing an `execute` function

pub mod gate;
pub mod ports;
pub mod results;
pub mod runner;
pub mod use_cases;
