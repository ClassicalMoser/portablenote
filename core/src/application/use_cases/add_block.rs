use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::application::ports::{BlockStore, Clock, NameIndex};
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockAdded;

/// Create a new block. Rejects duplicate names and IDs.
///
/// Multi-store: returns a `CommandResult` with `SaveBlock` and `SetName` writes.
/// The name is NFC-normalized before storage per spec §1.
pub fn execute(
    blocks: &dyn BlockStore,
    names: &dyn NameIndex,
    clock: &dyn Clock,
    id: Uuid,
    name: &str,
    content: &str,
) -> Result<CommandResult<BlockAdded>, DomainError> {
    let name: String = name.nfc().collect();

    if let Some((_, existing_id)) = names.resolve_ignore_case(&name) {
        if existing_id != id {
            return Err(DomainError::NameConflict(name, existing_id));
        }
    }
    if blocks.get(id).is_some() {
        return Err(DomainError::DuplicateId(id));
    }

    let block = blocks::create(id, &name, content, clock.now())?;

    Ok(CommandResult {
        writes: vec![
            VaultWrite::WriteBlock(block),
            VaultWrite::SetName { name: name.clone(), id },
        ],
        event: BlockAdded {
            block_id: id,
            name,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlockStore, MockClock, MockNameIndex};
    use crate::application::results::VaultWrite;
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

    fn mock_clock() -> MockClock {
        let mut clock = MockClock::new();
        clock.expect_now().returning(Utc::now);
        clock
    }

    #[test]
    fn happy_path_returns_writes_and_event() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();
        let clock = mock_clock();

        names
            .expect_resolve_ignore_case()
            .with(eq("Alpha"))
            .return_once(|_| None);
        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&blocks, &names, &clock, id(), "Alpha", "content").unwrap();

        assert_eq!(result.event.block_id, id());
        assert_eq!(result.event.name, "Alpha");
        assert_eq!(result.writes.len(), 2);
        assert!(matches!(&result.writes[0], VaultWrite::WriteBlock(b) if b.id == id() && b.name == "Alpha" && b.content == "content"));
        assert!(matches!(&result.writes[1], VaultWrite::SetName { name, id: wid } if name == "Alpha" && *wid == id()));
    }

    #[test]
    fn name_conflict_returns_error() {
        let blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve_ignore_case()
            .with(eq("Alpha"))
            .return_once(move |_| Some(("Alpha".to_string(), other())));

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Alpha", "content");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }

    #[test]
    fn name_conflict_case_insensitive_returns_error() {
        let blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve_ignore_case()
            .with(eq("welcome"))
            .return_once(move |_| Some(("Welcome".to_string(), other())));

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "welcome", "content");
        assert!(matches!(result, Err(DomainError::NameConflict(_, _))));
    }

    #[test]
    fn duplicate_id_returns_error() {
        use crate::domain::types::Block;
        use chrono::Utc;

        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve_ignore_case()
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

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Alpha", "content");
        assert!(matches!(result, Err(DomainError::DuplicateId(_))));
    }

    #[test]
    fn nfd_name_is_stored_as_nfc() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();
        let clock = mock_clock();

        // NFD input: "cafe\u{0301}" should become NFC "caf\u{00e9}" in writes
        let nfd_name = "cafe\u{0301}";
        let nfc_name = "caf\u{00e9}";

        names
            .expect_resolve_ignore_case()
            .return_once(|_| None);
        blocks
            .expect_get()
            .return_once(|_| None);

        let result = execute(&blocks, &names, &clock, id(), nfd_name, "content").unwrap();

        assert_eq!(result.event.name, nfc_name);
        assert!(matches!(&result.writes[0], VaultWrite::WriteBlock(b) if b.name == nfc_name));
        assert!(matches!(&result.writes[1], VaultWrite::SetName { name, .. } if name == nfc_name));
    }

    #[test]
    fn heading_in_content_returns_error() {
        let mut blocks = MockBlockStore::new();
        let mut names = MockNameIndex::new();

        names
            .expect_resolve_ignore_case()
            .with(eq("Alpha"))
            .return_once(|_| None);
        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let clock = mock_clock();
        let result = execute(&blocks, &names, &clock, id(), "Alpha", "## Bad");
        assert!(matches!(result, Err(DomainError::HeadingInContent)));
    }
}
