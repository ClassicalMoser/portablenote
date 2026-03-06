use uuid::Uuid;

use crate::application::ports::DocumentStore;
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionRemoved;

/// Remove a top-level section from a document. Fails if the section is not found.
///
/// Single-store: returns a `CommandResult` with a single `SaveDocument` write.
pub fn execute(
    doc_store: &dyn DocumentStore,
    document_id: Uuid,
    block_id: Uuid,
) -> Result<CommandResult<SectionRemoved>, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;

    let updated = documents::remove_section(doc, block_id)
        .ok_or(DomainError::SectionNotFound(block_id))?;

    Ok(CommandResult {
        writes: vec![VaultWrite::WriteDocument(updated)],
        event: SectionRemoved { document_id, block_id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockDocumentStore;
    use crate::application::results::VaultWrite;
    use crate::domain::types::{Document, Section};
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const S1: &str = "00000000-0000-4000-a000-000000000002";

    fn doc_id() -> Uuid { Uuid::parse_str(DOC).unwrap() }
    fn root() -> Uuid { Uuid::parse_str(ROOT).unwrap() }
    fn s1() -> Uuid { Uuid::parse_str(S1).unwrap() }
    fn make_doc_with_section() -> Document {
        Document {
            id: doc_id(), root: root(),
            sections: vec![Section { block: s1(), subsections: vec![] }],
        }
    }

    #[test]
    fn happy_path_returns_save_document_write() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc_with_section()));

        let result = execute(&docs, doc_id(), s1()).unwrap();

        assert_eq!(result.event.block_id, s1());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::WriteDocument(d) if d.sections.is_empty()));
    }

    #[test]
    fn section_not_found_returns_error() {
        let mut docs = MockDocumentStore::new();

        let missing = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc_with_section()));

        let result = execute(&docs, doc_id(), missing);
        assert!(matches!(result, Err(DomainError::SectionNotFound(_))));
    }

    #[test]
    fn document_not_found_returns_error() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| None);

        let result = execute(&docs, doc_id(), s1());
        assert!(matches!(result, Err(DomainError::DocumentNotFound(_))));
    }
}
