use uuid::Uuid;

use crate::domain::types::{Block, Document, Edge};

/// Read-only access to the block heap.
///
/// Use cases query through this trait. Writes are expressed as `VaultWrite`
/// entries in `CommandResult` and applied by the adapter — never by use cases.
#[cfg_attr(test, mockall::automock)]
pub trait BlockStore {
    fn get(&self, id: Uuid) -> Option<Block>;
    fn list(&self) -> Vec<Block>;
    /// Return all blocks whose content contains `[[name]]`.
    fn find_by_ref(&self, name: &str) -> Vec<Block>;
}

/// Read-only access to the reference graph.
#[cfg_attr(test, mockall::automock)]
pub trait GraphStore {
    fn get_edge(&self, id: Uuid) -> Option<Edge>;
    fn incoming(&self, block_id: Uuid) -> Vec<Edge>;
    /// All edges where `source == block_id` OR `target == block_id`.
    fn edges_for(&self, block_id: Uuid) -> Vec<Edge>;
}

/// Read-only access to document definitions.
#[cfg_attr(test, mockall::automock)]
pub trait DocumentStore {
    fn get(&self, id: Uuid) -> Option<Document>;
}

/// Read-only name → UUID index (vault-wide unique).
#[cfg_attr(test, mockall::automock)]
pub trait NameIndex {
    fn resolve(&self, name: &str) -> Option<Uuid>;
}
