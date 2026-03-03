use uuid::Uuid;

use crate::application::ports::DocumentStore;
use crate::domain::error::DomainError;
use crate::domain::events::DocumentDeleted;

pub fn execute(
    doc_store: &mut dyn DocumentStore,
    document_id: Uuid,
) -> Result<DocumentDeleted, DomainError> {
    if doc_store.get(document_id).is_none() {
        return Err(DomainError::DocumentNotFound(document_id));
    }
    doc_store.delete(document_id);
    Ok(DocumentDeleted { document_id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockDocumentStore;
    use crate::domain::types::Document;
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";

    fn doc_id() -> Uuid {
        Uuid::parse_str(DOC).unwrap()
    }
    fn root() -> Uuid {
        Uuid::parse_str(ROOT).unwrap()
    }

    #[test]
    fn happy_path_deletes_document() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(move |_| {
                Some(Document {
                    id: doc_id(),
                    root: root(),
                    sections: vec![],
                })
            });
        docs.expect_delete()
            .with(eq(doc_id()))
            .times(1)
            .return_once(|_| ());

        let event = execute(&mut docs, doc_id()).unwrap();
        assert_eq!(event.document_id, doc_id());
    }

    #[test]
    fn document_not_found_returns_error() {
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| None);

        let result = execute(&mut docs, doc_id());
        assert!(matches!(result, Err(DomainError::DocumentNotFound(_))));
    }
}
