use uuid::Uuid;

use crate::application::ports::{BlockStore, GraphStore};
use crate::application::results::DeleteBlockCascadeResult;
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockDeleted;

pub fn execute(
    block_store: &dyn BlockStore,
    graph: &dyn GraphStore,
    block_id: Uuid,
) -> Result<DeleteBlockCascadeResult, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    let referencing = block_store.find_by_ref(&block.name);
    let (reverted_blocks, inline_refs_reverted) =
        blocks::revert_refs(referencing, &block.name, block_id);

    let all_edges = graph.edges_for(block_id);
    let all_edge_ids: Vec<Uuid> = all_edges.iter().map(|e| e.id).collect();

    Ok(DeleteBlockCascadeResult {
        block_id,
        reverted_blocks,
        all_edge_ids,
        name_to_remove: block.name.clone(),
        event: BlockDeleted {
            block_id,
            edges_removed: all_edges.len(),
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
    const EDGE_IN: &str = "00000000-0000-4000-a000-0000000000e1";
    const EDGE_OUT: &str = "00000000-0000-4000-a000-0000000000e2";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn other() -> Uuid {
        Uuid::parse_str(OTHER).unwrap()
    }
    fn edge_in() -> Uuid {
        Uuid::parse_str(EDGE_IN).unwrap()
    }
    fn edge_out() -> Uuid {
        Uuid::parse_str(EDGE_OUT).unwrap()
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
    fn removes_all_edges_including_incoming() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(move |_| {
                vec![
                    Edge {
                        id: edge_in(),
                        source: other(),
                        target: id(),
                    },
                    Edge {
                        id: edge_out(),
                        source: id(),
                        target: other(),
                    },
                ]
            });

        let result = execute(&blocks, &graph, id()).unwrap();
        assert_eq!(result.all_edge_ids.len(), 2);
        assert!(result.all_edge_ids.contains(&edge_in()));
        assert!(result.all_edge_ids.contains(&edge_out()));
        assert_eq!(result.event.edges_removed, 2);
    }

    #[test]
    fn no_edges_returns_empty_list() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(|_| vec![]);

        let result = execute(&blocks, &graph, id()).unwrap();
        assert!(result.all_edge_ids.is_empty());
        assert_eq!(result.name_to_remove, "Alpha");
    }

    #[test]
    fn reverts_inline_refs_in_other_blocks() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        let referrer = Block {
            id: other(),
            name: "Referrer".to_string(),
            content: format!(
                "See [[Alpha]] here.\n\n<!-- refs -->\n[Alpha]: uuid:{ID}\n"
            ),
            created: Utc::now(),
            modified: Utc::now(),
        };

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(move |_| vec![referrer]);
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(|_| vec![]);

        let result = execute(&blocks, &graph, id()).unwrap();
        assert_eq!(result.reverted_blocks.len(), 1);
        assert!(result.reverted_blocks[0].content.contains("See Alpha here."));
        assert!(!result.reverted_blocks[0].content.contains("[[Alpha]]"));
        assert_eq!(result.event.inline_refs_reverted, 1);
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
