use uuid::Uuid;

use crate::application::ports::{BlockStore, Clock, DocumentStore, GraphStore};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::blocks;
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::BlockDeleted;

/// Force-delete a block, removing all edges (incoming + outgoing), reverting
/// inline references in other blocks to plain text, and removing the block
/// from every document that references it (section, subsection, or root).
///
/// Multi-store: returns a `CommandResult` with `WriteDocument`/`DeleteDocument`
/// for affected documents, `SaveBlock` for reverted blocks, `RemoveEdge`, `DeleteBlock`, and `RemoveName`.
pub fn execute(
    block_store: &dyn BlockStore,
    graph: &dyn GraphStore,
    documents: &dyn DocumentStore,
    clock: &dyn Clock,
    block_id: Uuid,
) -> Result<CommandResult<BlockDeleted>, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    let mut writes = Vec::new();

    // Remove block from every document that references it (section, subsection, or root).
    for doc_id in documents.list_ids() {
        if let Some(doc) = documents.get(doc_id) {
            if let Some(result) = documents::remove_block_from_document(doc, block_id) {
                match result {
                    documents::RemoveBlockResult::Updated(updated) => {
                        writes.push(VaultWrite::WriteDocument(updated));
                    }
                    documents::RemoveBlockResult::DeleteDocument => {
                        writes.push(VaultWrite::DeleteDocument(doc_id));
                    }
                }
            }
        }
    }

    let referencing = block_store.find_by_ref(&block.name);
    let (reverted_blocks, inline_refs_reverted) =
        blocks::revert_refs(referencing, &block.name, block_id, clock.now());

    let all_edges = graph.edges_for(block_id);

    for b in &reverted_blocks {
        writes.push(VaultWrite::WriteBlock(b.clone()));
    }
    for e in &all_edges {
        writes.push(VaultWrite::RemoveEdge(e.id));
    }
    writes.push(VaultWrite::DeleteBlock(block_id));
    writes.push(VaultWrite::RemoveName(block.name.clone()));

    Ok(CommandResult {
        writes,
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
    use crate::application::ports::{MockBlockStore, MockClock, MockDocumentStore, MockGraphStore};
    use crate::application::results::VaultWrite;
    use crate::domain::types::{Block, Edge};
    use chrono::Utc;
    use mockall::predicate::eq;

    const ID: &str = "00000000-0000-4000-a000-000000000001";
    const OTHER: &str = "00000000-0000-4000-a000-000000000002";
    const EDGE_IN: &str = "00000000-0000-4000-a000-0000000000e1";
    const EDGE_OUT: &str = "00000000-0000-4000-a000-0000000000e2";

    fn id() -> Uuid { Uuid::parse_str(ID).unwrap() }
    fn other() -> Uuid { Uuid::parse_str(OTHER).unwrap() }
    fn edge_in() -> Uuid { Uuid::parse_str(EDGE_IN).unwrap() }
    fn edge_out() -> Uuid { Uuid::parse_str(EDGE_OUT).unwrap() }
    fn make_block(id: Uuid, name: &str) -> Block {
        Block { id, name: name.to_string(), content: String::new(), created: Utc::now(), modified: Utc::now() }
    }
    fn mock_clock() -> MockClock {
        let mut c = MockClock::new();
        c.expect_now().returning(Utc::now);
        c
    }

    fn mock_documents_empty() -> MockDocumentStore {
        let mut docs = MockDocumentStore::new();
        docs.expect_list_ids().return_once(|| vec![]);
        docs
    }

    #[test]
    fn removes_all_edges_including_incoming() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let documents = mock_documents_empty();
        let clock = mock_clock();

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks.expect_find_by_ref().with(eq("Alpha")).return_once(|_| vec![]);
        graph.expect_edges_for().with(eq(id())).return_once(move |_| {
            vec![
                Edge { id: edge_in(), source: other(), target: id() },
                Edge { id: edge_out(), source: id(), target: other() },
            ]
        });

        let result = execute(&blocks, &graph, &documents, &clock, id()).unwrap();

        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::RemoveEdge(e) if *e == edge_in())));
        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::RemoveEdge(e) if *e == edge_out())));
        assert_eq!(result.event.edges_removed, 2);
    }

    #[test]
    fn no_edges_returns_delete_and_remove_name() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let documents = mock_documents_empty();

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks.expect_find_by_ref().with(eq("Alpha")).return_once(|_| vec![]);
        graph.expect_edges_for().with(eq(id())).return_once(|_| vec![]);

        let clock = mock_clock();
        let result = execute(&blocks, &graph, &documents, &clock, id()).unwrap();

        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::DeleteBlock(bid) if *bid == id())));
        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::RemoveName(n) if n == "Alpha")));
    }

    #[test]
    fn reverts_inline_refs_in_other_blocks() {
        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let documents = mock_documents_empty();

        let referrer = Block {
            id: other(),
            name: "Referrer".to_string(),
            content: format!("See [[Alpha]] here.\n\n<!-- refs -->\n[Alpha]: uuid:{ID}\n"),
            created: Utc::now(),
            modified: Utc::now(),
        };

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks.expect_find_by_ref().with(eq("Alpha")).return_once(move |_| vec![referrer]);
        graph.expect_edges_for().with(eq(id())).return_once(|_| vec![]);

        let clock = mock_clock();
        let result = execute(&blocks, &graph, &documents, &clock, id()).unwrap();

        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::WriteBlock(b) if b.content.contains("See Alpha here."))));
        assert_eq!(result.event.inline_refs_reverted, 1);
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let graph = MockGraphStore::new();
        let documents = mock_documents_empty();
        let clock = mock_clock();

        blocks.expect_get().with(eq(id())).return_once(|_| None);

        let result = execute(&blocks, &graph, &documents, &clock, id());
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }

    #[test]
    fn removes_block_from_document_section() {
        use crate::domain::types::{Document, Section};

        let doc_id = Uuid::parse_str("d0000000-0000-4000-a000-000000000001").unwrap();
        let doc = Document {
            id: doc_id,
            root: other(),
            sections: vec![Section { block: id(), subsections: vec![] }],
        };

        let mut blocks = MockBlockStore::new();
        let mut graph = MockGraphStore::new();
        let mut documents = MockDocumentStore::new();

        blocks.expect_get().with(eq(id())).return_once(move |_| Some(make_block(id(), "Alpha")));
        blocks.expect_find_by_ref().with(eq("Alpha")).return_once(|_| vec![]);
        graph.expect_edges_for().with(eq(id())).return_once(|_| vec![]);
        documents.expect_list_ids().return_once(move || vec![doc_id]);
        documents.expect_get().with(eq(doc_id)).return_once(move |_| Some(doc));

        let clock = mock_clock();
        let result = execute(&blocks, &graph, &documents, &clock, id()).unwrap();

        assert!(result.writes.iter().any(|w| matches!(w, VaultWrite::WriteDocument(d) if d.id == doc_id && d.sections.is_empty())));
    }
}
