mod common;
mod support;

use uuid::Uuid;

use portablenote_core::application::ports::{BlockStore, GraphStore, NameIndex};
use portablenote_core::application::results::VaultWrite;
use portablenote_core::application::use_cases::{
    add_block, add_edge, delete_block_safe, rename_block,
};
use portablenote_core::domain::error::DomainError;

use support::changeset;
use support::factory;

fn uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}

fn load_with_refs() -> support::in_memory::VaultStores {
    let dir = common::spec_dir().join("valid").join("with-refs");
    let vault = common::load_vault(&dir);
    factory::from_vault(&vault)
}

// ---------------------------------------------------------------------------
// add_block
// ---------------------------------------------------------------------------

#[test]
fn add_block_round_trip() {
    let mut stores = load_with_refs();
    let new_id = uuid("aaaaaaaa-0000-4000-a000-000000000001");

    assert_eq!(stores.blocks.list().len(), 3);
    assert!(stores.names.resolve("New Block").is_none());

    let result =
        add_block::execute(&stores.blocks, &stores.names, new_id, "New Block", "hello world")
            .unwrap();

    // Block is in the first write
    assert!(matches!(&result.writes[0], VaultWrite::SaveBlock(b) if b.id == new_id));
    assert_eq!(result.event.name, "New Block");

    changeset::apply_writes(&mut stores, result.writes);

    assert_eq!(stores.blocks.list().len(), 4);
    assert!(stores.blocks.get(new_id).is_some());
    assert_eq!(stores.names.resolve("New Block"), Some(new_id));
}

#[test]
fn add_block_rejects_duplicate_name() {
    let stores = load_with_refs();
    let new_id = uuid("aaaaaaaa-0000-4000-a000-000000000002");

    let result =
        add_block::execute(&stores.blocks, &stores.names, new_id, "Core Concepts", "dup");

    assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
}

// ---------------------------------------------------------------------------
// rename_block
// ---------------------------------------------------------------------------

#[test]
fn rename_block_propagates_refs() {
    let mut stores = load_with_refs();
    let getting_started_id = uuid("20000000-0000-4000-a000-000000000002");
    let core_concepts_id = uuid("20000000-0000-4000-a000-000000000001");

    let result = rename_block::execute(
        &stores.blocks,
        &stores.names,
        getting_started_id,
        "Quick Start",
    )
    .unwrap();

    // Renamed block is in the first write
    assert!(matches!(&result.writes[0], VaultWrite::SaveBlock(b) if b.name == "Quick Start"));
    assert_eq!(result.event.old_name, "Getting Started");
    assert_eq!(result.event.refs_updated, 1);

    changeset::apply_writes(&mut stores, result.writes);

    let renamed = stores.blocks.get(getting_started_id).unwrap();
    assert_eq!(renamed.name, "Quick Start");

    assert!(stores.names.resolve("Getting Started").is_none());
    assert_eq!(
        stores.names.resolve("Quick Start"),
        Some(getting_started_id)
    );

    let referrer = stores.blocks.get(core_concepts_id).unwrap();
    assert!(
        referrer.content.contains("[[Quick Start]]"),
        "inline ref should be updated: {}",
        referrer.content
    );
    assert!(
        !referrer.content.contains("[[Getting Started]]"),
        "old ref should be gone: {}",
        referrer.content
    );
}

// ---------------------------------------------------------------------------
// delete_block_safe
// ---------------------------------------------------------------------------

#[test]
fn delete_block_safe_rejects_with_incoming_edges() {
    let stores = load_with_refs();
    let getting_started_id = uuid("20000000-0000-4000-a000-000000000002");

    let result = delete_block_safe::execute(&stores.blocks, &stores.graph, getting_started_id);

    assert!(
        matches!(result, Err(DomainError::HasIncomingEdges(_, 1))),
        "should reject: Getting Started has 1 incoming edge from Core Concepts"
    );
}

#[test]
fn delete_block_safe_succeeds_for_source_only_block() {
    let mut stores = load_with_refs();
    let core_concepts_id = uuid("20000000-0000-4000-a000-000000000001");

    let result =
        delete_block_safe::execute(&stores.blocks, &stores.graph, core_concepts_id).unwrap();

    assert_eq!(result.event.block_id, core_concepts_id);
    let outgoing_count = result
        .writes
        .iter()
        .filter(|w| matches!(w, VaultWrite::RemoveEdge(_)))
        .count();
    assert_eq!(outgoing_count, 2, "Core Concepts has 2 outgoing edges");
    assert!(result
        .writes
        .iter()
        .any(|w| matches!(w, VaultWrite::RemoveName(n) if n == "Core Concepts")));

    changeset::apply_writes(&mut stores, result.writes);

    assert!(stores.blocks.get(core_concepts_id).is_none());
    assert!(stores.names.resolve("Core Concepts").is_none());
    assert_eq!(stores.blocks.list().len(), 2);
    assert!(
        stores.graph.edges_for(core_concepts_id).is_empty(),
        "all edges involving deleted block should be removed"
    );
}

// ---------------------------------------------------------------------------
// add_edge
// ---------------------------------------------------------------------------

#[test]
fn add_edge_round_trip() {
    let mut stores = load_with_refs();
    let new_edge_id = uuid("eeeeeeee-0000-4000-a000-000000000001");
    let getting_started_id = uuid("20000000-0000-4000-a000-000000000002");
    let advanced_id = uuid("20000000-0000-4000-a000-000000000003");

    assert_eq!(stores.graph.edges_for(getting_started_id).len(), 1);

    let result = add_edge::execute(
        &stores.blocks,
        &stores.graph,
        new_edge_id,
        getting_started_id,
        advanced_id,
    )
    .unwrap();

    assert_eq!(result.event.edge_id, new_edge_id);

    changeset::apply_writes(&mut stores, result.writes);

    assert!(stores.graph.get_edge(new_edge_id).is_some());
    assert_eq!(stores.graph.edges_for(getting_started_id).len(), 2);
}

#[test]
fn add_edge_rejects_dangling_target() {
    let stores = load_with_refs();
    let new_edge_id = uuid("eeeeeeee-0000-4000-a000-000000000002");
    let getting_started_id = uuid("20000000-0000-4000-a000-000000000002");
    let nonexistent = uuid("ffffffff-0000-4000-a000-ffffffffffff");

    let result = add_edge::execute(
        &stores.blocks,
        &stores.graph,
        new_edge_id,
        getting_started_id,
        nonexistent,
    );

    assert!(matches!(result, Err(DomainError::TargetNotInHeap(_))));
}
