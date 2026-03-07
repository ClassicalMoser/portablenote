//! Domain layer — pure logic, zero I/O.
//!
//! Modules are organized by aggregate or concern:
//!
//! - **`types`** — Core value objects and entities (`Block`, `Edge`, `Document`, etc.)
//! - **`blocks`**, **`edges`**, **`documents`** — Factory and mutation functions per aggregate
//! - **`content`** — Markdown content analysis (heading detection, inline ref extraction/renaming)
//! - **`format`** — Markdown block file parsing and serialization (`.md` ↔ `Block`). The system is markdown-native.
//! - **`commands`** / **`events`** — Command and event envelopes for CQRS-style dispatch
//! - **`error`** — Domain error types and invariant violations
//! - **`invariants`** — Full vault validation against spec rules
//! - **`checksum`** — Deterministic vault checksum computation
//! - **`queries`** — Read-only projections over a loaded vault

pub mod blocks;
pub mod checksum;
pub mod commands;
pub mod content;
pub mod documents;
pub mod edges;
pub mod error;
pub mod events;
pub mod format;
pub mod invariants;
pub mod queries;
pub mod types;
