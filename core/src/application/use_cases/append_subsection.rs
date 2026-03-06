use uuid::Uuid;

use crate::application::ports::{BlockStore, DocumentStore};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::documents;
use crate::domain::error::DomainError;
use crate::domain::events::SectionAppended;

/// Append a block as a subsection (depth 3) under an existing section.
///
/// Single-store: returns a `CommandResult` with a single `SaveDocument` write.
pub fn execute(
    block_store: &dyn BlockStore,
    doc_store: &dyn DocumentStore,
    document_id: Uuid,
    section_block_id: Uuid,
    block_id: Uuid,
) -> Result<CommandResult<SectionAppended>, DomainError> {
    let doc = doc_store
        .get(document_id)
        .ok_or(DomainError::DocumentNotFound(document_id))?;
    if block_store.get(block_id).is_none() {
        return Err(DomainError::BlockNotFound(block_id));
    }

    let updated = documents::append_subsection(doc, section_block_id, block_id)
        .ok_or(DomainError::SectionNotFound(section_block_id))?;

    Ok(CommandResult {
        writes: vec![VaultWrite::SaveDocument(updated)],
        event: SectionAppended { document_id, block_id, depth: 2 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockDocumentStore};
    use crate::application::results::VaultWrite;
    use crate::domain::types::{Block, Document, Section};
    use chrono::Utc;
    use mockall::predicate::eq;

    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";
    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const S1: &str = "00000000-0000-4000-a000-000000000002";
    const SUB: &str = "00000000-0000-4000-a000-000000000003";

    fn doc_id() -> Uuid { Uuid::parse_str(DOC).unwrap() }
    fn root() -> Uuid { Uuid::parse_str(ROOT).unwrap() }
    fn s1() -> Uuid { Uuid::parse_str(S1).unwrap() }
    fn sub() -> Uuid { Uuid::parse_str(SUB).unwrap() }
    fn make_doc_with_section() -> Document {
        Document {
            id: doc_id(), root: root(),
            sections: vec![Section { block: s1(), subsections: vec![] }],
        }
    }
    fn make_block(id: Uuid) -> Block {
        Block { id, name: "Block".to_string(), content: String::new(), created: Utc::now(), modified: Utc::now() }
    }

    #[test]
    fn happy_path_returns_save_document_write() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc_with_section()));
        blocks.expect_get().with(eq(sub())).return_once(move |_| Some(make_block(sub())));

        let result = execute(&blocks, &docs, doc_id(), s1(), sub()).unwrap();

        assert_eq!(result.event.depth, 2);
        assert_eq!(result.event.block_id, sub());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::SaveDocument(_)));
    }

    #[test]
    fn section_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut docs = MockDocumentStore::new();

        let missing = Uuid::parse_str("00000000-0000-4000-a000-ffffffffffff").unwrap();
        docs.expect_get().with(eq(doc_id())).return_once(|_| Some(make_doc_with_section()));
        blocks.expect_get().with(eq(sub())).return_once(move |_| Some(make_block(sub())));

        let result = execute(&blocks, &docs, doc_id(), missing, sub());
        assert!(matches!(result, Err(DomainError::SectionNotFound(_))));
    }
}
