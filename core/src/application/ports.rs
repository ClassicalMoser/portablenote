use uuid::Uuid;

use crate::domain::types::{Block, Document, Edge};

/// Read/write access to the block heap.
#[cfg_attr(test, mockall::automock)]
pub trait BlockStore {
    fn get(&self, id: Uuid) -> Option<Block>;
    fn list(&self) -> Vec<Block>;
    /// Return all blocks whose content contains `[[name]]`.
    fn find_by_ref(&self, name: &str) -> Vec<Block>;
    fn save(&mut self, block: &Block);
    fn save_all(&mut self, blocks: &[Block]);
    fn delete(&mut self, id: Uuid);
}

/// Read/write access to the reference graph.
#[cfg_attr(test, mockall::automock)]
pub trait GraphStore {
    fn get_edge(&self, id: Uuid) -> Option<Edge>;
    fn incoming(&self, block_id: Uuid) -> Vec<Edge>;
    /// All edges where `source == block_id` OR `target == block_id`.
    fn edges_for(&self, block_id: Uuid) -> Vec<Edge>;
    fn save_edge(&mut self, edge: &Edge);
    fn remove_edges(&mut self, ids: &[Uuid]);
}

/// Read/write access to document definitions.
#[cfg_attr(test, mockall::automock)]
pub trait DocumentStore {
    fn get(&self, id: Uuid) -> Option<Document>;
    fn save(&mut self, doc: &Document);
    fn delete(&mut self, id: Uuid);
}

/// Human-readable name → UUID index (vault-wide unique).
#[cfg_attr(test, mockall::automock)]
pub trait NameIndex {
    fn resolve(&self, name: &str) -> Option<Uuid>;
    fn set(&mut self, name: &str, id: Uuid);
    fn remove(&mut self, name: &str);
}
