use uuid::Uuid;

use crate::application::ports::{BlockStore, DocumentStore};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::DocumentAdded;

/// Create a new document rooted at the given block.
///
/// Single-store: returns a `CommandResult` with a single `SaveDocument` write.
pub fn execute(
    block_store: &dyn BlockStore,
    doc_store: &dyn DocumentStore,
    id: Uuid,
    root: Uuid,
) -> Result<CommandResult<DocumentAdded>, DomainError> {
    if block_store.get(root).is_none() {
        return Err(DomainError::RootNotInHeap(root));
    }
    if doc_store.get(id).is_some() {
        return Err(DomainError::DuplicateId(id));
    }

    let doc = documents::create(id, root);

    Ok(CommandResult {
        writes: vec![VaultWrite::WriteDocument(doc)],
        event: DocumentAdded { document_id: id, root_block_id: root },
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

    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";

    fn root() -> Uuid { Uuid::parse_str(ROOT).unwrap() }
    fn doc_id() -> Uuid { Uuid::parse_str(DOC).unwrap() }
    fn make_block(id: Uuid) -> Block {
        Block { id, name: "Root".to_string(), content: String::new(), created: Utc::now(), modified: Utc::now() }
    }

    #[test]
    fn happy_path_returns_save_document_write() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        blocks.expect_get().with(eq(root())).return_once(move |_| Some(make_block(root())));
        docs.expect_get().with(eq(doc_id())).return_once(|_| None);

        let result = execute(&blocks, &docs, doc_id(), root()).unwrap();

        assert_eq!(result.event.document_id, doc_id());
        assert_eq!(result.event.root_block_id, root());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::WriteDocument(d) if d.id == doc_id()));
    }

    #[test]
    fn root_not_in_heap_returns_error() {
        let mut blocks = MockBlockStore::new();
        let docs = MockDocumentStore::new();

        blocks.expect_get().with(eq(root())).return_once(|_| None);

        let result = execute(&blocks, &docs, doc_id(), root());
        assert!(matches!(result, Err(DomainError::RootNotInHeap(_))));
    }

    #[test]
    fn duplicate_document_id_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        blocks.expect_get().with(eq(root())).return_once(move |_| Some(make_block(root())));
        docs.expect_get().return_once(move |_| {
            Some(Document { id: doc_id(), root: root(), sections: vec![] })
        });

        let result = execute(&blocks, &docs, doc_id(), root());
        assert!(matches!(result, Err(DomainError::DuplicateId(_))));
    }
}
