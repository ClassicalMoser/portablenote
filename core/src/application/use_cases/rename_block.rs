use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::application::ports::{BlockStore, Clock, NameIndex};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockRenamed;

/// Rename a block and propagate `[[wikilink]]` updates to all referencing blocks.
///
/// Multi-store: returns a `CommandResult` with `SaveBlock` writes for the renamed
/// block and all propagated blocks, plus `RemoveName`/`SetName` index swaps.
/// The new name is NFC-normalized before storage per spec §1.
pub fn execute(
    block_store: &dyn BlockStore,
    names: &dyn NameIndex,
    clock: &dyn Clock,
    block_id: Uuid,
    new_name: &str,
) -> Result<CommandResult<BlockRenamed>, DomainError> {
    let new_name: String = new_name.nfc().collect();

    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    if let Some((_, existing_id)) = names.resolve_ignore_case(&new_name) {
        if existing_id != block_id {
            return Err(DomainError::NameConflict(new_name, existing_id));
        }
    }

    let old_name = block.name.clone();
    let now = clock.now();
    let renamed = blocks::apply_rename(block, &new_name, now)?;

    let referencing = block_store.find_by_ref(&old_name);
    let (propagated, refs_updated) = blocks::propagate_rename(referencing, &old_name, &new_name, now);

    let mut writes = Vec::new();
    writes.push(VaultWrite::WriteBlock(renamed.clone()));
    for b in &propagated {
        writes.push(VaultWrite::WriteBlock(b.clone()));
    }
    writes.push(VaultWrite::RemoveName(old_name.clone()));
    writes.push(VaultWrite::SetName { name: new_name.clone(), id: block_id });

    Ok(CommandResult {
        writes,
        event: BlockRenamed {
            block_id,
            old_name,
            new_name,
            refs_updated,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockClock, MockNameIndex};
    use crate::application::results::VaultWrite;
    use crate::domain::types::Block;
    use chrono::Utc;
    use mockall::predicate::eq;

    fn mock_clock() -> MockClock {
        let mut c = MockClock::new();
        c.expect_now().returning(Utc::now);
        c
    }

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
            .expect_resolve_ignore_case()
            .with(eq("Beta"))
            .return_once(|_| None);
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(|_| vec![]);

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Beta").unwrap();

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
            .expect_resolve_ignore_case()
            .with(eq("Beta"))
            .return_once(|_| None);
        blocks
            .expect_find_by_ref()
            .with(eq("Alpha"))
            .return_once(move |_| vec![referrer]);

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Beta").unwrap();

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

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Beta");
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
            .expect_resolve_ignore_case()
            .with(eq("Beta"))
            .return_once(move |_| Some(("Beta".to_string(), other())));

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Beta");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }

    #[test]
    fn nfd_new_name_is_stored_as_nfc() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        let nfd_name = "cafe\u{0301}";
        let nfc_name = "caf\u{00e9}";

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Alpha")));
        names
            .expect_resolve_ignore_case()
            .return_once(|_| None);
        blocks
            .expect_find_by_ref()
            .return_once(|_| vec![]);

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), nfd_name).unwrap();

        assert_eq!(result.event.new_name, nfc_name);
        assert!(matches!(&result.writes[0], VaultWrite::WriteBlock(b) if b.name == nfc_name));
        assert!(matches!(&result.writes.last().unwrap(), VaultWrite::SetName { name, .. } if name == nfc_name));
    }

    #[test]
    fn name_conflict_case_insensitive_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id(), "Core Concepts")));
        names
            .expect_resolve_ignore_case()
            .with(eq("welcome"))
            .return_once(move |_| Some(("Welcome".to_string(), other())));

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "welcome");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }
}
