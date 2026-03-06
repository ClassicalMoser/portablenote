use uuid::Uuid;

use crate::application::ports::{BlockStore, DocumentStore};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionAppended;

/// Append a block as a top-level section (depth 2) in a document.
///
/// Single-store: returns a `CommandResult` with a single `SaveDocument` write.
pub fn execute(
    block_store: &dyn BlockStore,
    doc_store: &dyn DocumentStore,
    document_id: Uuid,
    block_id: Uuid,
) -> Result<CommandResult<SectionAppended>, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;
    if block_store.get(block_id).is_none() {
        return Err(DomainError::BlockNotFound(block_id));
    }

    let updated = documents::append_section(doc, block_id);

    Ok(CommandResult {
        writes: vec![VaultWrite::SaveDocument(updated)],
        event: SectionAppended { document_id, block_id, depth: 1 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockDocumentStore};
    use crate::application::results::VaultWrite;
    use crate::domain::types::{Block, Document};
    use chrono::Utc;
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const BLOCK: &str = "00000000-0000-4000-a000-000000000002";

    fn doc_id() -> Uuid { Uuid::parse_str(DOC).unwrap() }
    fn root() -> Uuid { Uuid::parse_str(ROOT).unwrap() }
    fn block_id() -> Uuid { Uuid::parse_str(BLOCK).unwrap() }
    fn make_doc() -> Document { Document { id: doc_id(), root: root(), sections: vec![] } }
    fn make_block(id: Uuid) -> Block {
        Block { id, name: "Block".to_string(), content: String::new(), created: Utc::now(), modified: Utc::now() }
    }

    #[test]
    fn happy_path_returns_save_document_write() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc()));
        blocks.expect_get().with(eq(block_id())).return_once(move |_| Some(make_block(block_id())));

        let result = execute(&blocks, &docs, doc_id(), block_id()).unwrap();

        assert_eq!(result.event.depth, 1);
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::SaveDocument(d) if d.sections.len() == 1));
    }

    #[test]
    fn document_not_found_returns_error() {
        let blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| None);

        let result = execute(&blocks, &docs, doc_id(), block_id());
        assert!(matches!(result, Err(DomainError::DocumentNotFound(_))));
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc()));
        blocks.expect_get().with(eq(block_id())).return_once(|_| None);

        let result = execute(&blocks, &docs, doc_id(), block_id());
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }
}
