mod common;

use portablenote_core::queries;
use uuid::Uuid;

fn with_refs_vault() -> portablenote_core::types::Vault {
    let dir = common::spec_dir().join("valid").join("with-refs");
    common::load_vault(&dir)
}

fn with_orphans_vault() -> portablenote_core::types::Vault {
    let dir = common::spec_dir().join("valid").join("with-orphans");
    common::load_vault(&dir)
}

fn minimal_vault() -> portablenote_core::types::Vault {
    let dir = common::spec_dir().join("valid").join("minimal");
    common::load_vault(&dir)
}

#[test]
fn resolve_name_existing() {
    let vault = with_refs_vault();
    let id = queries::resolve_name(&vault, "Core Concepts");
    assert!(id.is_some());
    assert_eq!(
        id.unwrap(),
        Uuid::parse_str("20000000-0000-4000-a000-000000000001").unwrap()
    );
}

#[test]
fn resolve_name_missing() {
    let vault = with_refs_vault();
    let id = queries::resolve_name(&vault, "Nonexistent Block");
    assert!(id.is_none());
}

#[test]
fn edges_for_block_with_outgoing() {
    let vault = with_refs_vault();
    let core_id = Uuid::parse_str("20000000-0000-4000-a000-000000000001").unwrap();
    let (outgoing, incoming) = queries::edges_for(&vault, core_id);

    assert_eq!(outgoing.len(), 2, "Core Concepts has 2 outgoing edges");
    assert_eq!(incoming.len(), 0, "Core Concepts has 0 incoming edges");
}

#[test]
fn edges_for_block_with_incoming() {
    let vault = with_refs_vault();
    let getting_started_id = Uuid::parse_str("20000000-0000-4000-a000-000000000002").unwrap();
    let (outgoing, incoming) = queries::edges_for(&vault, getting_started_id);

    assert_eq!(outgoing.len(), 0);
    assert_eq!(incoming.len(), 1);
    assert_eq!(
        incoming[0].source,
        Uuid::parse_str("20000000-0000-4000-a000-000000000001").unwrap()
    );
}

#[test]
fn orphans_in_with_orphans_vault() {
    let vault = with_orphans_vault();
    let orphan_ids = queries::orphans(&vault);

    assert_eq!(orphan_ids.len(), 1, "Should have exactly one orphan");
    assert_eq!(
        orphan_ids[0],
        Uuid::parse_str("40000000-0000-4000-a000-000000000003").unwrap()
    );
}

#[test]
fn orphans_in_minimal_vault() {
    let vault = minimal_vault();
    let orphan_ids = queries::orphans(&vault);
    assert_eq!(orphan_ids.len(), 1, "Single block with no edges is an orphan");
}

#[test]
fn orphans_in_with_refs_vault() {
    let vault = with_refs_vault();
    let orphan_ids = queries::orphans(&vault);
    assert!(
        orphan_ids.is_empty(),
        "All blocks in with-refs are connected: {orphan_ids:?}"
    );
}

#[test]
fn list_blocks_count() {
    let vault = with_refs_vault();
    let blocks = queries::list_blocks(&vault);
    assert_eq!(blocks.len(), 3);
}

#[test]
fn backlinks_for_target() {
    let vault = with_refs_vault();
    let getting_started_id = Uuid::parse_str("20000000-0000-4000-a000-000000000002").unwrap();
    let sources = queries::backlinks(&vault, getting_started_id);

    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources[0],
        Uuid::parse_str("20000000-0000-4000-a000-000000000001").unwrap()
    );
}

#[test]
fn backlinks_for_block_with_none() {
    let vault = with_refs_vault();
    let core_id = Uuid::parse_str("20000000-0000-4000-a000-000000000001").unwrap();
    let sources = queries::backlinks(&vault, core_id);
    assert!(sources.is_empty());
}
