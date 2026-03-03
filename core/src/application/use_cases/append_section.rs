use uuid::Uuid;

use crate::application::ports::{BlockStore, DocumentStore};
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionAppended;

pub fn execute(
    block_store: &dyn BlockStore,
    doc_store: &mut dyn DocumentStore,
    document_id: Uuid,
    block_id: Uuid,
) -> Result<SectionAppended, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;
    if block_store.get(block_id).is_none() {
        return Err(DomainError::BlockNotFound(block_id));
    }

    let updated = documents::append_section(doc, block_id);
    doc_store.save(&updated);

    Ok(SectionAppended {
        document_id,
        block_id,
        depth: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockDocumentStore};
    use crate::domain::types::{Block, Document};
    use chrono::Utc;
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const BLOCK: &str = "00000000-0000-4000-a000-000000000002";

    fn doc_id() -> Uuid {
        Uuid::parse_str(DOC).unwrap()
    }
    fn root() -> Uuid {
        Uuid::parse_str(ROOT).unwrap()
    }
    fn block_id() -> Uuid {
        Uuid::parse_str(BLOCK).unwrap()
    }
    fn make_doc() -> Document {
        Document {
            id: doc_id(),
            root: root(),
            sections: vec![],
        }
    }
    fn make_block(id: Uuid) -> Block {
        Block {
            id,
            name: "Block".to_string(),
            content: String::new(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    #[test]
    fn happy_path_appends_section() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| Some(make_doc()));
        blocks
            .expect_get()
            .with(eq(block_id()))
            .return_once(move |_| Some(make_block(block_id())));
        docs.expect_save().times(1).return_once(|_| ());

        let event = execute(&blocks, &mut docs, doc_id(), block_id()).unwrap();
        assert_eq!(event.depth, 1);
    }

    #[test]
    fn document_not_found_returns_error() {
        let blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| None);

        let result = execute(&blocks, &mut docs, doc_id(), block_id());
        assert!(matches!(result, Err(DomainError::DocumentNotFound(_))));
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get()
            .with(eq(doc_id()))
            .return_once(|_| Some(make_doc()));
        blocks
            .expect_get()
            .with(eq(block_id()))
            .return_once(|_| None);

        let result = execute(&blocks, &mut docs, doc_id(), block_id());
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }
}
