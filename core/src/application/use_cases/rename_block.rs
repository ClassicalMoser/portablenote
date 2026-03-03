use uuid::Uuid;

use crate::application::ports::{BlockStore, NameIndex};
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockRenamed;

pub fn execute(
    block_store: &mut dyn BlockStore,
    names: &mut dyn NameIndex,
    block_id: Uuid,
    new_name: &str,
) -> Result<BlockRenamed, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    if let Some(existing) = names.resolve(new_name) {
        if existing != block_id {
            return Err(DomainError::NameConflict(new_name.to_string(), existing));
        }
    }

    let old_name = block.name.clone();
    let renamed = blocks::apply_rename(block, new_name)?;
    block_store.save(&renamed);

    let referencing = block_store.find_by_ref(&old_name);
    let (updated, refs_updated) = blocks::propagate_rename(referencing, &old_name, new_name);
    block_store.save_all(&updated);

    names.remove(&old_name);
    names.set(new_name, block_id);

    Ok(BlockRenamed {
        block_id,
        old_name,
        new_name: new_name.to_string(),
        refs_updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockNameIndex};
    use crate::domain::types::Block;
    use chrono::Utc;
    use mockall::predicate::eq;

    const ID: &str = "00000000-0000-4000-a000-000000000001";
    const OTHER: &str = "00000000-0000-4000-a000-000000000002";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn other() -> Uuid {
        Uuid::parse_str(OTHER).unwrap()
    }
    fn make_block(id: Uuid, name: &str) -> Block {
        Block {
            id,
            name: name.to_string(),
            content: String::new(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    #[test]
    fn happy_path_renames_and_propagates() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        names
            .expect_resolve()
            .with(eq("Beta"))
            .return_once(|_| None);
        blocks.expect_save().times(1).return_once(|_| ());
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);
        blocks.expect_save_all().times(1).return_once(|_| ());
        names
            .expect_remove()
            .with(eq("Alpha"))
            .times(1)
            .return_once(|_| ());
        names
            .expect_set()
            .with(eq("Beta"), eq(id()))
            .times(1)
            .return_once(|_, _| ());

        let event = execute(&mut blocks, &mut names, id(), "Beta").unwrap();
        assert_eq!(event.old_name, "Alpha");
        assert_eq!(event.new_name, "Beta");
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&mut blocks, &mut names, id(), "Beta");
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }

    #[test]
    fn name_conflict_with_different_block_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        names
            .expect_resolve()
            .with(eq("Beta"))
            .return_once(move |_| Some(other()));

        let result = execute(&mut blocks, &mut names, id(), "Beta");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }
}
