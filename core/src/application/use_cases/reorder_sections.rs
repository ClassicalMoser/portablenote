use uuid::Uuid;

use crate::application::ports::DocumentStore;
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionsReordered;

/// Reorder a document's top-level sections. The new order must contain exactly
/// the same block UUIDs as the current sections.
///
/// Single-store: returns a `CommandResult` with a single `SaveDocument` write.
pub fn execute(
    doc_store: &dyn DocumentStore,
    document_id: Uuid,
    section_order: Vec<Uuid>,
) -> Result<CommandResult<SectionsReordered>, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;

    let updated = documents::reorder_sections(doc, section_order)
        .ok_or(DomainError::InvalidSectionOrder)?;

    Ok(CommandResult {
        writes: vec![VaultWrite::WriteDocument(updated)],
        event: SectionsReordered { document_id },
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
    const S2: &str = "00000000-0000-4000-a000-000000000003";

    fn doc_id() -> Uuid { Uuid::parse_str(DOC).unwrap() }
    fn root() -> Uuid { Uuid::parse_str(ROOT).unwrap() }
    fn s1() -> Uuid { Uuid::parse_str(S1).unwrap() }
    fn s2() -> Uuid { Uuid::parse_str(S2).unwrap() }
    fn make_doc_two_sections() -> Document {
        Document {
            id: doc_id(), root: root(),
            sections: vec![
                Section { block: s1(), subsections: vec![] },
                Section { block: s2(), subsections: vec![] },
            ],
        }
    }

    #[test]
    fn happy_path_returns_save_document_write() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc_two_sections()));

        let result = execute(&docs, doc_id(), vec![s2(), s1()]).unwrap();

        assert_eq!(result.event.document_id, doc_id());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::WriteDocument(d) if d.sections[0].block == s2()));
    }

    #[test]
    fn wrong_section_set_returns_error() {
        let mut docs = MockDocumentStore::new();

        let bogus = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc_two_sections()));

        let result = execute(&docs, doc_id(), vec![s1(), bogus]);
        assert!(matches!(result, Err(DomainError::InvalidSectionOrder)));
    }
}
