use uuid::Uuid;

use crate::application::ports::{BlockStore, NameIndex};
use crate::application::results::RenameBlockResult;
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockRenamed;

pub fn execute(
    block_store: &dyn BlockStore,
    names: &dyn NameIndex,
    block_id: Uuid,
    new_name: &str,
) -> Result<RenameBlockResult, DomainError> {
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

    let referencing = block_store.find_by_ref(&old_name);
    let (propagated, refs_updated) = blocks::propagate_rename(referencing, &old_name, new_name);

    Ok(RenameBlockResult {
        renamed,
        propagated,
        old_name: old_name.clone(),
        event: BlockRenamed {
            block_id,
            old_name,
            new_name: new_name.to_string(),
            refs_updated,
        },
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
    fn happy_path_returns_renamed_and_propagated() {
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
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);

        let result = execute(&blocks, &names, id(), "Beta").unwrap();
        assert_eq!(result.renamed.name, "Beta");
        assert_eq!(result.old_name, "Alpha");
        assert!(result.propagated.is_empty());
        assert_eq!(result.event.old_name, "Alpha");
        assert_eq!(result.event.new_name, "Beta");
        assert_eq!(result.event.refs_updated, 0);
    }

    #[test]
    fn propagates_refs_in_other_blocks() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        let referrer_id = other();
        let referrer = Block {
            id: referrer_id,
            name: "Referrer".to_string(),
            content: "See [[Alpha]].\n\n<!-- refs -->\n[Alpha]: uuid:00000000-0000-4000-a000-000000000001\n".to_string(),
            created: Utc::now(),
            modified: Utc::now(),
        };

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        names
            .expect_resolve()
            .with(eq("Beta"))
            .return_once(|_| None);
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(move |_| vec![referrer]);

        let result = execute(&blocks, &names, id(), "Beta").unwrap();
        assert_eq!(result.propagated.len(), 1);
        assert!(result.propagated[0].content.contains("[[Beta]]"));
        assert_eq!(result.event.refs_updated, 1);
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();
        let names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&blocks, &names, id(), "Beta");
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

        let result = execute(&blocks, &names, id(), "Beta");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }
}
