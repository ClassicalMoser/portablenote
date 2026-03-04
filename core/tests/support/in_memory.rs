use std::collections::HashMap;

use uuid::Uuid;

use portablenote_core::application::ports::{BlockStore, DocumentStore, GraphStore, NameIndex};
use portablenote_core::domain::types::{Block, Document, Edge};

/// In-memory block heap. Keyed by block UUID.
pub struct InMemoryBlockStore {
    pub blocks: HashMap<Uuid, Block>,
}

impl BlockStore for InMemoryBlockStore {
    fn get(&self, id: Uuid) -> Option<Block> {
        self.blocks.get(&id).cloned()
    }

    fn list(&self) -> Vec<Block> {
        self.blocks.values().cloned().collect()
    }

    fn find_by_ref(&self, name: &str) -> Vec<Block> {
        let pattern = format!("[[{name}]]");
        self.blocks
            .values()
            .filter(|b| b.content.contains(&pattern))
            .cloned()
            .collect()
    }

    fn save(&mut self, block: &Block) {
        self.blocks.insert(block.id, block.clone());
    }

    fn save_all(&mut self, blocks: &[Block]) {
        for block in blocks {
            self.blocks.insert(block.id, block.clone());
        }
    }

    fn delete(&mut self, id: Uuid) {
        self.blocks.remove(&id);
    }
}

/// In-memory reference graph. Stores edges in a Vec for simplicity.
pub struct InMemoryGraphStore {
    pub edges: Vec<Edge>,
}

impl GraphStore for InMemoryGraphStore {
    fn get_edge(&self, id: Uuid) -> Option<Edge> {
        self.edges.iter().find(|e| e.id == id).cloned()
    }

    fn incoming(&self, block_id: Uuid) -> Vec<Edge> {
        self.edges
            .iter()
            .filter(|e| e.target == block_id)
            .cloned()
            .collect()
    }

    fn edges_for(&self, block_id: Uuid) -> Vec<Edge> {
        self.edges
            .iter()
            .filter(|e| e.source == block_id || e.target == block_id)
            .cloned()
            .collect()
    }

    fn save_edge(&mut self, edge: &Edge) {
        self.edges.retain(|e| e.id != edge.id);
        self.edges.push(edge.clone());
    }

    fn remove_edges(&mut self, ids: &[Uuid]) {
        self.edges.retain(|e| !ids.contains(&e.id));
    }
}

/// In-memory document store. Keyed by document UUID.
pub struct InMemoryDocumentStore {
    pub docs: HashMap<Uuid, Document>,
}

impl DocumentStore for InMemoryDocumentStore {
    fn get(&self, id: Uuid) -> Option<Document> {
        self.docs.get(&id).cloned()
    }

    fn save(&mut self, doc: &Document) {
        self.docs.insert(doc.id, doc.clone());
    }

    fn delete(&mut self, id: Uuid) {
        self.docs.remove(&id);
    }
}

/// In-memory name-to-UUID index. Case-preserving storage; lookups are exact.
/// Case-insensitive uniqueness is enforced by domain invariants, not here.
pub struct InMemoryNameIndex {
    pub names: HashMap<String, Uuid>,
}

impl NameIndex for InMemoryNameIndex {
    fn resolve(&self, name: &str) -> Option<Uuid> {
        self.names.get(name).copied()
    }

    fn set(&mut self, name: &str, id: Uuid) {
        self.names.insert(name.to_string(), id);
    }

    fn remove(&mut self, name: &str) {
        self.names.remove(name);
    }
}

/// Convenience bundle of all four stores for a single vault.
/// Fields are public for direct assertion access in tests.
#[allow(dead_code)] // individual fields used progressively as scenarios are added
pub struct VaultStores {
    pub blocks: InMemoryBlockStore,
    pub graph: InMemoryGraphStore,
    pub documents: InMemoryDocumentStore,
    pub names: InMemoryNameIndex,
}
