use uuid::Uuid;

use crate::application::ports::GraphStore;
use crate::domain::error::DomainError;
use crate::domain::events::EdgeRemoved;

pub fn execute(
    graph: &mut dyn GraphStore,
    edge_id: Uuid,
) -> Result<EdgeRemoved, DomainError> {
    if graph.get_edge(edge_id).is_none() {
        return Err(DomainError::EdgeNotFound(edge_id));
    }
    graph.remove_edges(&[edge_id]);
    Ok(EdgeRemoved { edge_id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockGraphStore;
    use crate::domain::types::Edge;
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

    #[test]
    fn happy_path_removes_edge() {
        let mut graph = MockGraphStore::new();

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
        graph.expect_remove_edges().times(1).return_once(|_| ());

        let event = execute(&mut graph, edge_id()).unwrap();
        assert_eq!(event.edge_id, edge_id());
    }

    #[test]
    fn edge_not_found_returns_error() {
        let mut graph = MockGraphStore::new();

        graph
            .expect_get_edge()
            .with(eq(edge_id()))
            .return_once(|_| None);

        let result = execute(&mut graph, edge_id());
        assert!(matches!(result, Err(DomainError::EdgeNotFound(_))));
    }
}
