use uuid::Uuid;

use crate::application::ports::{BlockStore, NameIndex};
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockAdded;

pub fn execute(
    blocks: &mut dyn BlockStore,
    names: &mut dyn NameIndex,
    id: Uuid,
    name: &str,
    content: &str,
) -> Result<BlockAdded, DomainError> {
    if let Some(existing) = names.resolve(name) {
        return Err(DomainError::NameConflict(name.to_string(), existing));
    }
    if blocks.get(id).is_some() {
        return Err(DomainError::DuplicateId(id));
    }

    let block = blocks::create(id, name, content)?;
    blocks.save(&block);
    names.set(name, id);

    Ok(BlockAdded {
        block_id: id,
        name: name.to_string(),
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
    fn happy_path_creates_block() {
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
        blocks.expect_save().times(1).return_once(|_| ());
        names
            .expect_set()
            .with(eq("Alpha"), eq(id()))
            .times(1)
            .return_once(|_, _| ());

        let event = execute(&mut blocks, &mut names, id(), "Alpha", "content").unwrap();
        assert_eq!(event.block_id, id());
        assert_eq!(event.name, "Alpha");
    }

    #[test]
    fn name_conflict_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve()
            .with(eq("Alpha"))
            .return_once(move |_| Some(other()));

        let result = execute(&mut blocks, &mut names, id(), "Alpha", "content");
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

        let result = execute(&mut blocks, &mut names, id(), "Alpha", "content");
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

        let result = execute(&mut blocks, &mut names, id(), "Alpha", "## Bad");
        assert!(matches!(result, Err(DomainError::HeadingInContent)));
    }
}
