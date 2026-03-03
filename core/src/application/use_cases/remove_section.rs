use uuid::Uuid;

use crate::application::ports::DocumentStore;
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionRemoved;

pub fn execute(
    doc_store: &mut dyn DocumentStore,
    document_id: Uuid,
    block_id: Uuid,
) -> Result<SectionRemoved, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;

    let updated = documents::remove_section(doc, block_id)
        .ok_or(DomainError::SectionNotFound(block_id))?;
    doc_store.save(&updated);

    Ok(SectionRemoved {
        document_id,
        block_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockDocumentStore;
    use crate::domain::types::{Document, Section};
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const S1: &str = "00000000-0000-4000-a000-000000000002";

    fn doc_id() -> Uuid {
        Uuid::parse_str(DOC).unwrap()
    }
    fn root() -> Uuid {
        Uuid::parse_str(ROOT).unwrap()
    }
    fn s1() -> Uuid {
        Uuid::parse_str(S1).unwrap()
    }
    fn make_doc_with_section() -> Document {
        Document {
            id: doc_id(),
            root: root(),
            sections: vec![Section {
                block: s1(),
                subsections: vec![],
            }],
        }
    }

    #[test]
    fn happy_path_removes_section() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| Some(make_doc_with_section()));
        docs.expect_save().times(1).return_once(|_| ());

        let event = execute(&mut docs, doc_id(), s1()).unwrap();
        assert_eq!(event.block_id, s1());
    }

    #[test]
    fn section_not_found_returns_error() {
        let mut docs = MockDocumentStore::new();

        let missing = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| Some(make_doc_with_section()));

        let result = execute(&mut docs, doc_id(), missing);
        assert!(matches!(result, Err(DomainError::SectionNotFound(_))));
    }

    #[test]
    fn document_not_found_returns_error() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| None);

        let result = execute(&mut docs, doc_id(), s1());
        assert!(matches!(result, Err(DomainError::DocumentNotFound(_))));
    }
}
