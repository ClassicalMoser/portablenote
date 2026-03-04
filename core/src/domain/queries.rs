use std::collections::HashSet;

use uuid::Uuid;

use super::types::{Block, Edge, Vault};

/// Resolve a block name to its UUID via the names index.
pub fn resolve_name(vault: &Vault, name: &str) -> Option<Uuid> {
    vault.names.get(name).copied()
}

/// Return (outgoing, incoming) edges for a given block UUID.
pub fn edges_for(vault: &Vault, block_id: Uuid) -> (Vec<&Edge>, Vec<&Edge>) {
    let outgoing: Vec<&Edge> = vault
        .graph
        .edges
        .iter()
        .filter(|e| e.source == block_id)
        .collect();

    let incoming: Vec<&Edge> = vault
        .graph
        .edges
        .iter()
        .filter(|e| e.target == block_id)
        .collect();

    (outgoing, incoming)
}

/// Return UUIDs of all orphan blocks (no incoming or outgoing edges).
pub fn orphans(vault: &Vault) -> Vec<Uuid> {
    let mut connected: HashSet<Uuid> = HashSet::new();

    for edge in &vault.graph.edges {
        connected.insert(edge.source);
        connected.insert(edge.target);
    }

    vault
        .blocks
        .keys()
        .filter(|id| !connected.contains(id))
        .copied()
        .collect()
}

/// List all blocks in the vault.
pub fn list_blocks(vault: &Vault) -> Vec<&Block> {
    vault.blocks.values().collect()
}

/// Return UUIDs of all blocks that have an edge pointing to the given block.
pub fn backlinks(vault: &Vault, block_id: Uuid) -> Vec<Uuid> {
    vault
        .graph
        .edges
        .iter()
        .filter(|e| e.target == block_id)
        .map(|e| e.source)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{BlockGraph, Manifest};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_block(id: Uuid, name: &str) -> Block {
        Block {
            id,
            name: name.to_string(),
            content: String::new(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    fn make_edge(id: Uuid, source: Uuid, target: Uuid) -> Edge {
        Edge { id, source, target }
    }

    fn empty_vault() -> Vault {
        Vault {
            manifest: Manifest {
                vault_id: Uuid::nil(),
                spec_version: "0.1.0".to_string(),
                format: "portablenote".to_string(),
                checksum: String::new(),
            },
            blocks: HashMap::new(),
            graph: BlockGraph {
                version: "0.1.0".to_string(),
                edges: Vec::new(),
            },
            documents: HashMap::new(),
            names: HashMap::new(),
            version: 0,
        }
    }

    fn two_block_vault() -> Vault {
        let a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let edge_id = Uuid::parse_str("00000000-0000-4000-a000-0000000000e1").unwrap();

        let mut names = HashMap::new();
        names.insert("Alpha".to_string(), a);
        names.insert("Beta".to_string(), b);

        let mut blocks = HashMap::new();
        blocks.insert(a, make_block(a, "Alpha"));
        blocks.insert(b, make_block(b, "Beta"));

        Vault {
            manifest: Manifest {
                vault_id: Uuid::nil(),
                spec_version: "0.1.0".to_string(),
                format: "portablenote".to_string(),
                checksum: String::new(),
            },
            blocks,
            graph: BlockGraph {
                version: "0.1.0".to_string(),
                edges: vec![make_edge(edge_id, a, b)],
            },
            documents: HashMap::new(),
            names,
            version: 0,
        }
    }

    #[test]
    fn resolve_name_hit() {
        let vault = two_block_vault();
        let id = resolve_name(&vault, "Alpha").unwrap();
        assert_eq!(
            id,
            Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap()
        );
    }

    #[test]
    fn resolve_name_miss() {
        let vault = two_block_vault();
        assert!(resolve_name(&vault, "Nonexistent").is_none());
    }

    #[test]
    fn resolve_name_empty_vault() {
        let vault = empty_vault();
        assert!(resolve_name(&vault, "Anything").is_none());
    }

    #[test]
    fn edges_for_source() {
        let vault = two_block_vault();
        let a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let (out, inc) = edges_for(&vault, a);
        assert_eq!(out.len(), 1);
        assert_eq!(inc.len(), 0);
    }

    #[test]
    fn edges_for_target() {
        let vault = two_block_vault();
        let b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let (out, inc) = edges_for(&vault, b);
        assert_eq!(out.len(), 0);
        assert_eq!(inc.len(), 1);
    }

    #[test]
    fn edges_for_unconnected() {
        let vault = empty_vault();
        let bogus = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        let (out, inc) = edges_for(&vault, bogus);
        assert!(out.is_empty());
        assert!(inc.is_empty());
    }

    #[test]
    fn orphans_all_connected() {
        let vault = two_block_vault();
        assert!(orphans(&vault).is_empty());
    }

    #[test]
    fn orphans_with_isolated_block() {
        let mut vault = two_block_vault();
        let c = Uuid::parse_str("00000000-0000-4000-a000-000000000003").unwrap();
        vault.blocks.insert(c, make_block(c, "Gamma"));
        let result = orphans(&vault);
        assert_eq!(result, vec![c]);
    }

    #[test]
    fn orphans_empty_vault() {
        let vault = empty_vault();
        assert!(orphans(&vault).is_empty());
    }

    #[test]
    fn list_blocks_count() {
        let vault = two_block_vault();
        assert_eq!(list_blocks(&vault).len(), 2);
    }

    #[test]
    fn list_blocks_empty() {
        let vault = empty_vault();
        assert!(list_blocks(&vault).is_empty());
    }

    #[test]
    fn backlinks_with_incoming() {
        let vault = two_block_vault();
        let a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let sources = backlinks(&vault, b);
        assert_eq!(sources, vec![a]);
    }

    #[test]
    fn backlinks_with_none() {
        let vault = two_block_vault();
        let a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        assert!(backlinks(&vault, a).is_empty());
    }

    #[test]
    fn backlinks_unknown_block() {
        let vault = two_block_vault();
        let bogus = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        assert!(backlinks(&vault, bogus).is_empty());
    }
}
