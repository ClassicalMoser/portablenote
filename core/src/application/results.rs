//! Mutation output types for all use cases.
//!
//! Every use case returns `CommandResult<E>` — a complete, ordered list of
//! `VaultWrite` operations to apply atomically, plus the domain event. No use
//! case writes to any store; adapters consume the writes and persist them.

use uuid::Uuid;

use crate::domain::types::{Block, Document, Edge};

/// Every possible mutation to vault state.
///
/// Adapters receive a `Vec<VaultWrite>` from a use case and apply all entries
/// atomically. The order is significant: writes must be applied in sequence to
/// maintain referential consistency (e.g. save reverted blocks before deleting
/// the block they referenced).
#[derive(Debug, Clone)]
pub enum VaultWrite {
    WriteBlock(Block),
    DeleteBlock(Uuid),
    WriteEdge(Edge),
    RemoveEdge(Uuid),
    WriteDocument(Document),
    DeleteDocument(Uuid),
    SetName { name: String, id: Uuid },
    RemoveName(String),
}

/// The output of every use case: an ordered set of writes to apply atomically,
/// plus the domain event describing what happened.
pub struct CommandResult<E> {
    pub writes: Vec<VaultWrite>,
    pub event: E,
}
