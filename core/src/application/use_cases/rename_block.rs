use uuid::Uuid;

use crate::application::ports::{BlockStore, NameIndex};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockRenamed;

/// Rename a block and propagate `[[wikilink]]` updates to all referencing blocks.
///
/// Multi-store: returns a `CommandResult` with `SaveBlock` writes for the renamed
/// block and all propagated blocks, plus `RemoveName`/`SetName` index swaps.
pub fn execute(
    block_store: &dyn BlockStore,
    names: &dyn NameIndex,
    block_id: Uuid,
    new_name: &str,
) -> Result<CommandResult<BlockRenamed>, DomainError> {
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

    let mut writes = Vec::new();
    writes.push(VaultWrite::WriteBlock(renamed.clone()));
    for b in &propagated {
        writes.push(VaultWrite::WriteBlock(b.clone()));
    }
    writes.push(VaultWrite::RemoveName(old_name.clone()));
    writes.push(VaultWrite::SetName { name: new_name.to_string(), id: block_id });

    Ok(CommandResult {
        writes,
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
    use crate::application::results::VaultWrite;
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
    fn happy_path_returns_writes_and_event() {
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

        assert_eq!(result.event.old_name, "Alpha");
        assert_eq!(result.event.new_name, "Beta");
        assert_eq!(result.event.refs_updated, 0);

        // SaveBlock(renamed), RemoveName(old), SetName(new)
        assert_eq!(result.writes.len(), 3);
        assert!(matches!(&result.writes[0], VaultWrite::WriteBlock(b) if b.name == "Beta"));
        assert!(matches!(&result.writes[1], VaultWrite::RemoveName(n) if n == "Alpha"));
        assert!(matches!(&result.writes[2], VaultWrite::SetName { name, .. } if name == "Beta"));
    }

    #[test]
    fn propagates_refs_in_other_blocks() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        let referrer = Block {
            id: other(),
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

        // SaveBlock(renamed), SaveBlock(propagated), RemoveName, SetName
        assert_eq!(result.writes.len(), 4);
        assert!(matches!(&result.writes[1], VaultWrite::WriteBlock(b) if b.content.contains("[[Beta]]")));
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
