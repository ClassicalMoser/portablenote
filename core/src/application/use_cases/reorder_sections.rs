use uuid::Uuid;

use crate::application::ports::DocumentStore;
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionsReordered;

pub fn execute(
    doc_store: &mut dyn DocumentStore,
    document_id: Uuid,
    section_order: Vec<Uuid>,
) -> Result<SectionsReordered, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;

    let updated = documents::reorder_sections(doc, section_order)
        .ok_or(DomainError::InvalidSectionOrder)?;
    doc_store.save(&updated);

    Ok(SectionsReordered { document_id })
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
    const S2: &str = "00000000-0000-4000-a000-000000000003";

    fn doc_id() -> Uuid {
        Uuid::parse_str(DOC).unwrap()
    }
    fn root() -> Uuid {
        Uuid::parse_str(ROOT).unwrap()
    }
    fn s1() -> Uuid {
        Uuid::parse_str(S1).unwrap()
    }
    fn s2() -> Uuid {
        Uuid::parse_str(S2).unwrap()
    }
    fn make_doc_two_sections() -> Document {
        Document {
            id: doc_id(),
            root: root(),
            sections: vec![
                Section {
                    block: s1(),
                    subsections: vec![],
                },
                Section {
                    block: s2(),
                    subsections: vec![],
                },
            ],
        }
    }

    #[test]
    fn happy_path_reorders_sections() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| Some(make_doc_two_sections()));
        docs.expect_save().times(1).return_once(|_| ());

        let event = execute(&mut docs, doc_id(), vec![s2(), s1()]).unwrap();
        assert_eq!(event.document_id, doc_id());
    }

    #[test]
    fn wrong_section_set_returns_error() {
        let mut docs = MockDocumentStore::new();

        let bogus = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| Some(make_doc_two_sections()));

        let result = execute(&mut docs, doc_id(), vec![s1(), bogus]);
        assert!(matches!(result, Err(DomainError::InvalidSectionOrder)));
    }
}
