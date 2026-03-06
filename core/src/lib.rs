//! # portablenote-core
//!
//! Pure domain and application logic for the PortableNote format. This crate
//! contains no I/O — it defines the entities, invariants, commands, events,
//! and use cases that any adapter (filesystem, REST, WASM) can drive.
//!
//! ## Architecture
//!
//! - **`domain`** — Value objects, entities, pure functions, and format parsing.
//!   Everything here is deterministic and side-effect-free (except `Utc::now()`
//!   for timestamps).
//! - **`application`** — Port traits (the hexagonal boundary), result types for
//!   multi-store coordination, and use case functions that orchestrate domain
//!   logic against those ports.
//!
//! See `core/ARCHITECTURE.md` for the full design rationale.

pub mod application;
pub mod domain;
