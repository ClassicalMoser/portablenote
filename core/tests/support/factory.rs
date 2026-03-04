use portablenote_core::domain::types::Vault;

use super::in_memory::{
    InMemoryBlockStore, InMemoryDocumentStore, InMemoryGraphStore, InMemoryNameIndex, VaultStores,
};

/// Hydrate a full set of in-memory stores from a loaded `Vault` snapshot.
/// This bridges the existing test loader (which produces a `Vault`) to the
/// port-based architecture (which operates on individual stores).
pub fn from_vault(vault: &Vault) -> VaultStores {
    VaultStores {
        blocks: InMemoryBlockStore {
            blocks: vault.blocks.clone(),
        },
        graph: InMemoryGraphStore {
            edges: vault.graph.edges.clone(),
        },
        documents: InMemoryDocumentStore {
            docs: vault.documents.clone(),
        },
        names: InMemoryNameIndex {
            names: vault.manifest.names.clone(),
        },
    }
}
