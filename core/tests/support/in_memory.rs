#![allow(dead_code)] // shared test infra — not every binary uses every type

use std::collections::HashMap;

use uuid::Uuid;

use portablenote_core::application::ports::{BlockStore, DocumentStore, GraphStore, NameIndex};
use portablenote_core::domain::types::{Block, Document, Edge};

/// In-memory block heap. Keyed by block UUID.
pub struct InMemoryBlockStore {
    pub blocks: HashMap<Uuid, Block>,
}

impl InMemoryBlockStore {
    pub fn save(&mut self, block: &Block) {
        self.blocks.insert(block.id, block.clone());
    }

    pub fn save_all(&mut self, blocks: &[Block]) {
        for block in blocks {
            self.blocks.insert(block.id, block.clone());
        }
    }

    pub fn delete(&mut self, id: Uuid) {
        self.blocks.remove(&id);
    }
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
}

/// In-memory reference graph. Stores edges in a Vec for simplicity.
pub struct InMemoryGraphStore {
    pub edges: Vec<Edge>,
}

impl InMemoryGraphStore {
    pub fn save_edge(&mut self, edge: &Edge) {
        self.edges.retain(|e| e.id != edge.id);
        self.edges.push(edge.clone());
    }

    pub fn remove_edge(&mut self, id: Uuid) {
        self.edges.retain(|e| e.id != id);
    }
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
}

/// In-memory document store. Keyed by document UUID.
pub struct InMemoryDocumentStore {
    pub docs: HashMap<Uuid, Document>,
}

impl InMemoryDocumentStore {
    pub fn save(&mut self, doc: &Document) {
        self.docs.insert(doc.id, doc.clone());
    }

    pub fn delete(&mut self, id: Uuid) {
        self.docs.remove(&id);
    }
}

impl DocumentStore for InMemoryDocumentStore {
    fn get(&self, id: Uuid) -> Option<Document> {
        self.docs.get(&id).cloned()
    }

    fn list_ids(&self) -> Vec<Uuid> {
        self.docs.keys().cloned().collect()
    }
}

/// In-memory name-to-UUID index.
pub struct InMemoryNameIndex {
    pub names: HashMap<String, Uuid>,
}

impl InMemoryNameIndex {
    pub fn set(&mut self, name: &str, id: Uuid) {
        self.names.insert(name.to_string(), id);
    }

    pub fn remove(&mut self, name: &str) {
        self.names.remove(name);
    }
}

impl NameIndex for InMemoryNameIndex {
    fn resolve(&self, name: &str) -> Option<Uuid> {
        self.names.get(name).copied()
    }

    fn resolve_ignore_case(&self, name: &str) -> Option<(String, Uuid)> {
        let key = name.to_lowercase();
        self.names
            .iter()
            .find(|(k, _)| k.to_lowercase() == key)
            .map(|(k, v)| (k.clone(), *v))
    }
}

/// Convenience bundle of all four stores for a single vault.
/// Fields are public for direct assertion access in tests.
pub struct VaultStores {
    pub blocks: InMemoryBlockStore,
    pub graph: InMemoryGraphStore,
    pub documents: InMemoryDocumentStore,
    pub names: InMemoryNameIndex,
}
