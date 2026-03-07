//! Filesystem adapter for `MutationGate`. Builds a vault snapshot from the
//! FS stores and runs the core gate rule.

use std::collections::HashMap;

use portablenote_core::application::gate;
use portablenote_core::application::ports::{BlockStore, ManifestStore, MutationGate};
use portablenote_core::domain::error::DomainError;
use portablenote_core::domain::types::Vault;

use super::{FsBlockStore, FsDocumentStore, FsGraphStore, FsManifestStore, FsNameIndex};

/// Mutation gate backed by the filesystem stores. Builds a full vault snapshot
/// from the stores and runs the core gate (§5).
pub struct FsMutationGate<'a> {
    pub blocks: &'a FsBlockStore,
    pub graph: &'a FsGraphStore,
    pub documents: &'a FsDocumentStore,
    pub names: &'a FsNameIndex,
    pub manifest: &'a FsManifestStore,
}

impl FsMutationGate<'_> {
    /// Build a full vault snapshot from current store state (for commit or gate).
    pub fn build_vault(&self) -> Option<Vault> {
        let manifest = self.manifest.get()?;
        let blocks: HashMap<_, _> = self
            .blocks
            .list()
            .into_iter()
            .map(|b| (b.id, b))
            .collect();
        let graph = self.graph.as_block_graph().clone();
        let documents = self.documents.all_documents().clone();
        let names = self.names.all_names().clone();
        Some(Vault {
            manifest,
            blocks,
            graph,
            documents,
            names,
            version: 0,
        })
    }
}

impl MutationGate for FsMutationGate<'_> {
    fn allow_mutation(
        &self,
        expected_checksum: Option<String>,
    ) -> Result<(), DomainError> {
        let Some(vault) = self.build_vault() else {
            return Ok(());
        };
        gate::mutation_gate(&vault, expected_checksum.as_deref())
    }
}
