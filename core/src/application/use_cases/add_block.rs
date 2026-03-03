use uuid::Uuid;

use crate::application::ports::{BlockStore, NameIndex};
use crate::application::results::AddBlockResult;
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockAdded;

pub fn execute(
    blocks: &dyn BlockStore,
    names: &dyn NameIndex,
    id: Uuid,
    name: &str,
    content: &str,
) -> Result<AddBlockResult, DomainError> {
    if let Some(existing) = names.resolve(name) {
        return Err(DomainError::NameConflict(name.to_string(), existing));
    }
    if blocks.get(id).is_some() {
        return Err(DomainError::DuplicateId(id));
    }

    let block = blocks::create(id, name, content)?;

    Ok(AddBlockResult {
        block,
        event: BlockAdded {
            block_id: id,
            name: name.to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockNameIndex};
    use mockall::predicate::eq;

    const ID: &str = "00000000-0000-4000-a000-000000000001";
    const OTHER: &str = "00000000-0000-4000-a000-000000000002";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn other() -> Uuid {
        Uuid::parse_str(OTHER).unwrap()
    }

    #[test]
    fn happy_path_returns_block_and_event() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve()
            .with(eq("Alpha"))
            .return_once(|_| None);
        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&blocks, &names, id(), "Alpha", "content").unwrap();
        assert_eq!(result.block.id, id());
        assert_eq!(result.block.name, "Alpha");
        assert_eq!(result.block.content, "content");
        assert_eq!(result.event.block_id, id());
        assert_eq!(result.event.name, "Alpha");
    }

    #[test]
    fn name_conflict_returns_error() {
        let blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve()
            .with(eq("Alpha"))
            .return_once(move |_| Some(other()));

        let result = execute(&blocks, &names, id(), "Alpha", "content");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }

    #[test]
    fn duplicate_id_returns_error() {
        use crate::domain::types::Block;
        use chrono::Utc;

        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve()
            .with(eq("Alpha"))
            .return_once(|_| None);
        blocks.expect_get().return_once(|_| {
            Some(Block {
                id: id(),
                name: "Existing".to_string(),
                content: String::new(),
                created: Utc::now(),
                modified: Utc::now(),
            })
        });

        let result = execute(&blocks, &names, id(), "Alpha", "content");
        assert!(matches!(result, Err(DomainError::DuplicateId(_))));
    }

    #[test]
    fn heading_in_content_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve()
            .with(eq("Alpha"))
            .return_once(|_| None);
        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&blocks, &names, id(), "Alpha", "## Bad");
        assert!(matches!(result, Err(DomainError::HeadingInContent)));
    }
}
