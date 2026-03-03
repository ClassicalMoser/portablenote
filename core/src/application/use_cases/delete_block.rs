use uuid::Uuid;

use crate::application::ports::{BlockStore, GraphStore, NameIndex};
use crate::domain::blocks;
use crate::domain::commands::DeleteMode;
use crate::domain::error::DomainError;
use crate::domain::events::BlockDeleted;

pub fn execute(
    block_store: &mut dyn BlockStore,
    graph: &mut dyn GraphStore,
    names: &mut dyn NameIndex,
    block_id: Uuid,
    mode: DeleteMode,
) -> Result<BlockDeleted, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    let incoming = graph.incoming(block_id);
    if mode == DeleteMode::Safe && !incoming.is_empty() {
        return Err(DomainError::HasIncomingEdges(block_id, incoming.len()));
    }

    let referencing = block_store.find_by_ref(&block.name);
    let (reverted, inline_refs_reverted) = blocks::revert_refs(referencing, &block.name, block_id);
    block_store.save_all(&reverted);

    let all_edges = graph.edges_for(block_id);
    let edge_ids: Vec<Uuid> = all_edges.iter().map(|e| e.id).collect();
    let edges_removed = edge_ids.len();
    graph.remove_edges(&edge_ids);

    block_store.delete(block_id);
    names.remove(&block.name);

    Ok(BlockDeleted {
        block_id,
        edges_removed,
        inline_refs_reverted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockGraphStore, MockNameIndex};
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
    fn make_edge(id: Uuid, source: Uuid, target: Uuid) -> Edge {
        Edge { id, source, target }
    }

    #[test]
    fn safe_delete_no_incoming_succeeds() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let mut names = MockNameIndex::new();

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
        blocks.expect_save_all().times(1).return_once(|_| ());
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(|_| vec![]);
        graph.expect_remove_edges().times(1).return_once(|_| ());
        blocks
            .expect_delete()
            .with(eq(id()))
            .times(1)
            .return_once(|_| ());
        names
            .expect_remove()
            .with(eq("Alpha"))
            .times(1)
            .return_once(|_| ());

        let event =
            execute(&mut blocks, &mut graph, &mut names, id(), DeleteMode::Safe).unwrap();
        assert_eq!(event.block_id, id());
        assert_eq!(event.edges_removed, 0);
    }

    #[test]
    fn safe_delete_with_incoming_is_rejected() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        graph
            .expect_incoming()
            .with(eq(id()))
            .return_once(move |_| vec![make_edge(edge_id(), other(), id())]);

        let result = execute(&mut blocks, &mut graph, &mut names, id(), DeleteMode::Safe);
        assert!(matches!(result, Err(DomainError::HasIncomingEdges(_, 1))));
    }

    #[test]
    fn cascade_delete_removes_incoming_edges() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        graph
            .expect_incoming()
            .with(eq(id()))
            .return_once(move |_| vec![make_edge(edge_id(), other(), id())]);
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);
        blocks.expect_save_all().times(1).return_once(|_| ());
        graph
            .expect_edges_for()
            .with(eq(id()))
            .return_once(move |_| vec![make_edge(edge_id(), other(), id())]);
        graph.expect_remove_edges().times(1).return_once(|_| ());
        blocks
            .expect_delete()
            .with(eq(id()))
            .times(1)
            .return_once(|_| ());
        names
            .expect_remove()
            .with(eq("Alpha"))
            .times(1)
            .return_once(|_| ());

        let event =
            execute(&mut blocks, &mut graph, &mut names, id(), DeleteMode::Cascade).unwrap();
        assert_eq!(event.edges_removed, 1);
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&mut blocks, &mut graph, &mut names, id(), DeleteMode::Safe);
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }
}
