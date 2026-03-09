use uuid::Uuid;

use crate::application::block_file;
use crate::application::ports::{BlockStore, Clock, GraphStore};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::error::DomainError;
use crate::domain::events::BlockDeleted;
use crate::domain::types::Block;

/// Delete a block, rejected if it has incoming edges.
///
/// Outgoing edges and block-reference links to this block in other blocks are
/// reverted to plain text. Multi-store: returns a `CommandResult` with `SaveBlock` writes
/// for reverted blocks, `RemoveEdge` for outgoing edges, `DeleteBlock`, and `RemoveName`.
pub fn execute(
    block_store: &dyn BlockStore,
    graph: &dyn GraphStore,
    clock: &dyn Clock,
    block_id: Uuid,
) -> Result<CommandResult<BlockDeleted>, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    let incoming = graph.incoming(block_id);
    if !incoming.is_empty() {
        return Err(DomainError::HasIncomingEdges(block_id, incoming.len()));
    }

    let referencing = block_store.find_by_target(block_id);
    let now = clock.now();
    let mut inline_refs_reverted = 0usize;
    let reverted_blocks: Vec<Block> = referencing
        .into_iter()
        .map(|mut b| {
            let (new_content, count) = block_file::revert_refs_in_content(&b.content, block_id);
            if count > 0 {
                inline_refs_reverted += count;
                b.content = new_content;
                b.modified = now;
            }
            b
        })
        .collect();

    let outgoing = graph.edges_for(block_id);

    let mut writes = Vec::new();
    for b in &reverted_blocks {
        writes.push(VaultWrite::WriteBlock(b.clone()));
    }
    for e in &outgoing {
        writes.push(VaultWrite::RemoveEdge(e.id));
    }
    writes.push(VaultWrite::DeleteBlock(block_id));
    writes.push(VaultWrite::RemoveName(block.name.clone()));

    Ok(CommandResult {
        writes,
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
    use crate::application::ports::{MockBlockStore, MockClock, MockGraphStore};
    use crate::application::results::VaultWrite;
    use crate::domain::types::{Block, Edge};
    use chrono::Utc;
    use mockall::predicate::eq;

    const ID: &str = "00000000-0000-4000-a000-000000000001";
    const OTHER: &str = "00000000-0000-4000-a000-000000000002";
    const EDGE: &str = "00000000-0000-4000-a000-0000000000e1";

    fn id() -> Uuid { Uuid::parse_str(ID).unwrap() }
    fn other() -> Uuid { Uuid::parse_str(OTHER).unwrap() }
    fn edge_id() -> Uuid { Uuid::parse_str(EDGE).unwrap() }
    fn make_block(id: Uuid, name: &str) -> Block {
        Block { id, name: name.to_string(), content: String::new(), created: Utc::now(), modified: Utc::now() }
    }
    fn mock_clock() -> MockClock {
        let mut c = MockClock::new();
        c.expect_now().returning(Utc::now);
        c
    }

    #[test]
    fn no_incoming_returns_result() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let clock = mock_clock();

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        graph.expect_incoming().with(eq(id())).return_once(|_| vec![]);
        blocks.expect_find_by_target().with(eq(id())).return_once(|_| vec![]);
        graph.expect_edges_for().with(eq(id())).return_once(|_| vec![]);

        let result = execute(&blocks, &graph, &clock, id()).unwrap();

        assert_eq!(result.event.block_id, id());
        // DeleteBlock + RemoveName
        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::DeleteBlock(bid) if *bid == id())));
        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::RemoveName(n) if n == "Alpha")));
    }

    #[test]
    fn outgoing_edges_included_as_remove_writes() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        graph.expect_incoming().with(eq(id())).return_once(|_| vec![]);
        blocks.expect_find_by_target().with(eq(id())).return_once(|_| vec![]);
        graph.expect_edges_for().with(eq(id())).return_once(move |_| {
            vec![Edge { id: edge_id(), source: id(), target: other() }]
        });

        let clock = mock_clock();
        let result = execute(&blocks, &graph, &clock, id()).unwrap();

        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::RemoveEdge(eid) if *eid == edge_id())));
        assert_eq!(result.event.edges_removed, 1);
    }

    #[test]
    fn incoming_edges_rejected() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        graph.expect_incoming().with(eq(id())).return_once(move |_| {
            vec![Edge { id: edge_id(), source: other(), target: id() }]
        });

        let clock = mock_clock();
        let result = execute(&blocks, &graph, &clock, id());
        assert!(matches!(result, Err(DomainError::HasIncomingEdges(_, 1))));
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let graph = MockGraphStore::new();
        let clock = mock_clock();

        blocks.expect_get().with(eq(id())).return_once(|_| None);

        let result = execute(&blocks, &graph, &clock, id());
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }
}
