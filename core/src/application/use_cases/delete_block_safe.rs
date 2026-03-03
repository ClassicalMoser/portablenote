use uuid::Uuid;

use crate::application::ports::{BlockStore, GraphStore};
use crate::application::results::DeleteBlockSafeResult;
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockDeleted;

pub fn execute(
    block_store: &dyn BlockStore,
    graph: &dyn GraphStore,
    block_id: Uuid,
) -> Result<DeleteBlockSafeResult, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    let incoming = graph.incoming(block_id);
    if !incoming.is_empty() {
        return Err(DomainError::HasIncomingEdges(block_id, incoming.len()));
    }

    let referencing = block_store.find_by_ref(&block.name);
    let (reverted_blocks, inline_refs_reverted) =
        blocks::revert_refs(referencing, &block.name, block_id);

    let outgoing = graph.edges_for(block_id);
    let outgoing_edge_ids: Vec<Uuid> = outgoing.iter().map(|e| e.id).collect();

    Ok(DeleteBlockSafeResult {
        block_id,
        reverted_blocks,
        outgoing_edge_ids,
        name_to_remove: block.name.clone(),
        event: BlockDeleted {
            block_id,
            edges_removed: outgoing.len(),
            inline_refs_reverted,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockGraphStore};
    use crate::domain::types::{Block, Edge};
    use chrono::Utc;
    use mockall::predicate::eq;

    const ID: &str = "00000000-0000-4000-a000-000000000001";
    const OTHER: &str = "00000000-0000-4000-a000-000000000002";
    const EDGE: &str = "00000000-0000-4000-a000-0000000000e1";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn other() -> Uuid {
        Uuid::parse_str(OTHER).unwrap()
    }
    fn edge_id() -> Uuid {
        Uuid::parse_str(EDGE).unwrap()
    }
    fn make_block(id: Uuid, name: &str) -> Block {
        Block {
            id,
            name: name.to_string(),
            content: String::new(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    #[test]
    fn no_incoming_returns_result() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        graph
            .expect_incoming()
            .with(eq(id()))
            .return_once(|_| vec![]);
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(|_| vec![]);

        let result = execute(&blocks, &graph, id()).unwrap();
        assert_eq!(result.block_id, id());
        assert_eq!(result.name_to_remove, "Alpha");
        assert!(result.reverted_blocks.is_empty());
        assert!(result.outgoing_edge_ids.is_empty());
    }

    #[test]
    fn outgoing_edges_included_in_result() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        graph
            .expect_incoming()
            .with(eq(id()))
            .return_once(|_| vec![]);
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(move |_| {
                vec![Edge {
                    id: edge_id(),
                    source: id(),
                    target: other(),
                }]
            });

        let result = execute(&blocks, &graph, id()).unwrap();
        assert_eq!(result.outgoing_edge_ids, vec![edge_id()]);
        assert_eq!(result.event.edges_removed, 1);
    }

    #[test]
    fn incoming_edges_rejected() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        graph
            .expect_incoming()
            .with(eq(id()))
            .return_once(move |_| {
                vec![Edge {
                    id: edge_id(),
                    source: other(),
                    target: id(),
                }]
            });

        let result = execute(&blocks, &graph, id());
        assert!(matches!(result, Err(DomainError::HasIncomingEdges(_, 1))));
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&blocks, &graph, id());
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }
}
