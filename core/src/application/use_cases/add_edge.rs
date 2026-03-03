use uuid::Uuid;

use crate::application::ports::{BlockStore, GraphStore};
use crate::domain::edges;
use crate::domain::error::DomainError;
use crate::domain::events::EdgeAdded;

pub fn execute(
    block_store: &dyn BlockStore,
    graph: &mut dyn GraphStore,
    id: Uuid,
    source: Uuid,
    target: Uuid,
) -> Result<EdgeAdded, DomainError> {
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
    graph.save_edge(&edge);

    Ok(EdgeAdded {
        edge_id: id,
        source,
        target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockGraphStore};
    use crate::domain::types::{Block, Edge};
    use chrono::Utc;
    use mockall::predicate::eq;

    const SRC: &str = "00000000-0000-4000-a000-000000000001";
    const TGT: &str = "00000000-0000-4000-a000-000000000002";
    const EDGE: &str = "00000000-0000-4000-a000-0000000000e1";

    fn src() -> Uuid {
        Uuid::parse_str(SRC).unwrap()
    }
    fn tgt() -> Uuid {
        Uuid::parse_str(TGT).unwrap()
    }
    fn edge_id() -> Uuid {
        Uuid::parse_str(EDGE).unwrap()
    }
    fn make_block(id: Uuid) -> Block {
        Block {
            id,
            name: "Block".to_string(),
            content: String::new(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    #[test]
    fn happy_path_creates_edge() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(src()))
            .return_once(move |_| Some(make_block(src())));
        blocks
            .expect_get()
            .with(eq(tgt()))
            .return_once(move |_| Some(make_block(tgt())));
        graph
            .expect_get_edge()
            .with(eq(edge_id()))
            .return_once(|_| None);
        graph.expect_save_edge().times(1).return_once(|_| ());

        let event = execute(&blocks, &mut graph, edge_id(), src(), tgt()).unwrap();
        assert_eq!(event.edge_id, edge_id());
        assert_eq!(event.source, src());
        assert_eq!(event.target, tgt());
    }

    #[test]
    fn source_not_in_heap_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(src()))
            .return_once(|_| None);

        let result = execute(&blocks, &mut graph, edge_id(), src(), tgt());
        assert!(matches!(result, Err(DomainError::SourceNotInHeap(_))));
    }

    #[test]
    fn target_not_in_heap_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(src()))
            .return_once(move |_| Some(make_block(src())));
        blocks
            .expect_get()
            .with(eq(tgt()))
            .return_once(|_| None);

        let result = execute(&blocks, &mut graph, edge_id(), src(), tgt());
        assert!(matches!(result, Err(DomainError::TargetNotInHeap(_))));
    }

    #[test]
    fn duplicate_edge_id_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks
            .expect_get()
            .with(eq(src()))
            .return_once(move |_| Some(make_block(src())));
        blocks
            .expect_get()
            .with(eq(tgt()))
            .return_once(move |_| Some(make_block(tgt())));
        graph
            .expect_get_edge()
            .with(eq(edge_id()))
            .return_once(move |_| {
                Some(Edge {
                    id: edge_id(),
                    source: src(),
                    target: tgt(),
                })
            });

        let result = execute(&blocks, &mut graph, edge_id(), src(), tgt());
        assert!(matches!(result, Err(DomainError::DuplicateId(_))));
    }
}
