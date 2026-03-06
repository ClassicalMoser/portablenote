use uuid::Uuid;

use crate::application::ports::GraphStore;
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::error::DomainError;
use crate::domain::events::EdgeRemoved;

/// Remove an edge from the reference graph. Fails if the edge does not exist.
///
/// Single-store: returns a `CommandResult` with a single `RemoveEdge` write.
pub fn execute(
    graph: &dyn GraphStore,
    edge_id: Uuid,
) -> Result<CommandResult<EdgeRemoved>, DomainError> {
    if graph.get_edge(edge_id).is_none() {
        return Err(DomainError::EdgeNotFound(edge_id));
    }

    Ok(CommandResult {
        writes: vec![VaultWrite::RemoveEdge(edge_id)],
        event: EdgeRemoved { edge_id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockGraphStore;
    use crate::application::results::VaultWrite;
    use crate::domain::types::Edge;
    use mockall::predicate::eq;

    const SRC: &str = "00000000-0000-4000-a000-000000000001";
    const TGT: &str = "00000000-0000-4000-a000-000000000002";
    const EDGE: &str = "00000000-0000-4000-a000-0000000000e1";

    fn src() -> Uuid { Uuid::parse_str(SRC).unwrap() }
    fn tgt() -> Uuid { Uuid::parse_str(TGT).unwrap() }
    fn edge_id() -> Uuid { Uuid::parse_str(EDGE).unwrap() }

    #[test]
    fn happy_path_returns_remove_edge_write() {
        let mut graph = MockGraphStore::new();

        graph.expect_get_edge().with(eq(edge_id())).return_once(move |_| {
            Some(Edge { id: edge_id(), source: src(), target: tgt() })
        });

        let result = execute(&graph, edge_id()).unwrap();

        assert_eq!(result.event.edge_id, edge_id());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::RemoveEdge(eid) if *eid == edge_id()));
    }

    #[test]
    fn edge_not_found_returns_error() {
        let mut graph = MockGraphStore::new();

        graph.expect_get_edge().with(eq(edge_id())).return_once(|_| None);

        let result = execute(&graph, edge_id());
        assert!(matches!(result, Err(DomainError::EdgeNotFound(_))));
    }
}
