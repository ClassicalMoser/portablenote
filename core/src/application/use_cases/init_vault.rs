//! InitVault: create a new vault (genesis manifest, empty graph, empty names).
//!
//! No ports required — no prior state. The adapter writes the returned
//! manifest, graph, and empty names to the target path.

use uuid::Uuid;

use crate::application::results::InitVaultResult;
use crate::domain::events::VaultInitialized;
use crate::domain::types::{BlockGraph, Manifest};

/// SHA-256 of empty input (no blocks, edges, or documents in canonical serialization).
const GENESIS_CHECKSUM: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Produce the genesis vault state. The caller writes `manifest` to
/// `portablenote.json`, `graph` to `block-graph.json`, and `{}` to `names.json`.
pub fn execute(vault_id: Option<Uuid>) -> InitVaultResult {
    let vault_id = vault_id.unwrap_or_else(Uuid::new_v4);
    let manifest = Manifest {
        vault_id,
        spec_version: "0.1.0".to_string(),
        format: "markdown".to_string(),
        checksum: GENESIS_CHECKSUM.to_string(),
        previous_checksum: None,
    };
    let graph = BlockGraph {
        version: "0.1.0".to_string(),
        edges: vec![],
    };
    InitVaultResult {
        manifest,
        graph,
        event: VaultInitialized { vault_id },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_vault_returns_genesis_manifest_and_empty_graph() {
        let id = Uuid::new_v4();
        let result = execute(Some(id));
        assert_eq!(result.manifest.vault_id, id);
        assert_eq!(result.manifest.checksum, GENESIS_CHECKSUM);
        assert!(result.manifest.previous_checksum.is_none());
        assert!(result.graph.edges.is_empty());
        assert_eq!(result.event.vault_id, id);
    }

    #[test]
    fn init_vault_generates_vault_id_when_none_given() {
        let result = execute(None);
        assert_ne!(result.manifest.vault_id, Uuid::nil());
        assert_eq!(result.manifest.vault_id, result.event.vault_id);
    }
}
