use uuid::Uuid;

use crate::application::ports::DocumentStore;
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::error::DomainError;
use crate::domain::events::DocumentDeleted;

/// Delete a document definition. Does not affect blocks or the graph.
///
/// Single-store: returns a `CommandResult` with a single `DeleteDocument` write.
pub fn execute(
    doc_store: &dyn DocumentStore,
    document_id: Uuid,
) -> Result<CommandResult<DocumentDeleted>, DomainError> {
    if doc_store.get(document_id).is_none() {
        return Err(DomainError::DocumentNotFound(document_id));
    }

    Ok(CommandResult {
        writes: vec![VaultWrite::DeleteDocument(document_id)],
        event: DocumentDeleted { document_id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockDocumentStore;
    use crate::application::results::VaultWrite;
    use crate::domain::types::Document;
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";

    fn doc_id() -> Uuid { Uuid::parse_str(DOC).unwrap() }
    fn root() -> Uuid { Uuid::parse_str(ROOT).unwrap() }

    #[test]
    fn happy_path_returns_delete_document_write() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(move |_| {
            Some(Document { id: doc_id(), root: root(), sections: vec![] })
        });

        let result = execute(&docs, doc_id()).unwrap();

        assert_eq!(result.event.document_id, doc_id());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::DeleteDocument(did) if *did == doc_id()));
    }

    #[test]
    fn document_not_found_returns_error() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| None);

        let result = execute(&docs, doc_id());
        assert!(matches!(result, Err(DomainError::DocumentNotFound(_))));
    }
}
