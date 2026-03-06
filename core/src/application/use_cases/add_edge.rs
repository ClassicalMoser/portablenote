use uuid::Uuid;

use crate::application::ports::{BlockStore, GraphStore};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::edges;
use crate::domain::error::DomainError;
use crate::domain::events::EdgeAdded;

/// Create a directed edge between two blocks. Both endpoints must exist.
///
/// Single-store: returns a `CommandResult` with a single `SaveEdge` write.
pub fn execute(
    block_store: &dyn BlockStore,
    graph: &dyn GraphStore,
    id: Uuid,
    source: Uuid,
    target: Uuid,
) -> Result<CommandResult<EdgeAdded>, DomainError> {
    if block_store.get(source).is_none() {
        return Err(DomainError::SourceNotInHeap(source));
    }
    if block_store.get(target).is_none() {
        return Err(DomainError::TargetNotInHeap(target));
    }
    if graph.get_edge(id).is_some() {
        return Err(DomainError::DuplicateId(id));
    }

    let edge = edges::create(id, source, target);

    Ok(CommandResult {
        writes: vec![VaultWrite::SaveEdge(edge)],
        event: EdgeAdded { edge_id: id, source, target },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockGraphStore};
    use crate::application::results::VaultWrite;
    use crate::domain::types::{Block, Edge};
    use chrono::Utc;
    use mockall::predicate::eq;

    const SRC: &str = "00000000-0000-4000-a000-000000000001";
    const TGT: &str = "00000000-0000-4000-a000-000000000002";
    const EDGE: &str = "00000000-0000-4000-a000-0000000000e1";

    fn src() -> Uuid { Uuid::parse_str(SRC).unwrap() }
    fn tgt() -> Uuid { Uuid::parse_str(TGT).unwrap() }
    fn edge_id() -> Uuid { Uuid::parse_str(EDGE).unwrap() }
    fn make_block(id: Uuid) -> Block {
        Block { id, name: "Block".to_string(), content: String::new(), created: Utc::now(), modified: Utc::now() }
    }

    #[test]
    fn happy_path_returns_save_edge_write() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks.expect_get().with(eq(src())).return_once(move |_| Some(make_block(src())));
        blocks.expect_get().with(eq(tgt())).return_once(move |_| Some(make_block(tgt())));
        graph.expect_get_edge().with(eq(edge_id())).return_once(|_| None);

        let result = execute(&blocks, &graph, edge_id(), src(), tgt()).unwrap();

        assert_eq!(result.event.edge_id, edge_id());
        assert_eq!(result.event.source, src());
        assert_eq!(result.event.target, tgt());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::SaveEdge(e) if e.id == edge_id()));
    }

    #[test]
    fn source_not_in_heap_returns_error() {
        let mut blocks = MockBlockStore::new();
        let graph = MockGraphStore::new();

        blocks.expect_get().with(eq(src())).return_once(|_| None);

        let result = execute(&blocks, &graph, edge_id(), src(), tgt());
        assert!(matches!(result, Err(DomainError::SourceNotInHeap(_))));
    }

    #[test]
    fn target_not_in_heap_returns_error() {
        let mut blocks = MockBlockStore::new();
        let graph = MockGraphStore::new();

        blocks.expect_get().with(eq(src())).return_once(move |_| Some(make_block(src())));
        blocks.expect_get().with(eq(tgt())).return_once(|_| None);

        let result = execute(&blocks, &graph, edge_id(), src(), tgt());
        assert!(matches!(result, Err(DomainError::TargetNotInHeap(_))));
    }

    #[test]
    fn duplicate_edge_id_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks.expect_get().with(eq(src())).return_once(move |_| Some(make_block(src())));
        blocks.expect_get().with(eq(tgt())).return_once(move |_| Some(make_block(tgt())));
        graph.expect_get_edge().with(eq(edge_id())).return_once(move |_| {
            Some(Edge { id: edge_id(), source: src(), target: tgt() })
        });

        let result = execute(&blocks, &graph, edge_id(), src(), tgt());
        assert!(matches!(result, Err(DomainError::DuplicateId(_))));
    }
}
