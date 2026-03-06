use uuid::Uuid;

use crate::application::ports::BlockStore;
use crate::application::results::{CommandResult, VaultWrite};
use crate::domain::blocks;
use crate::domain::error::DomainError;
use crate::domain::events::BlockContentMutated;

/// Replace a block's content body. Validates the no-heading invariant.
///
/// Single-store: returns a `CommandResult` with a single `SaveBlock` write.
pub fn execute(
    block_store: &dyn BlockStore,
    block_id: Uuid,
    content: &str,
) -> Result<CommandResult<BlockContentMutated>, DomainError> {
    let block = block_store
        .get(block_id)
        .ok_or(DomainError::BlockNotFound(block_id))?;

    let updated = blocks::apply_content(block, content)?;

    Ok(CommandResult {
        writes: vec![VaultWrite::SaveBlock(updated)],
        event: BlockContentMutated { block_id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockBlockStore;
    use crate::application::results::VaultWrite;
    use crate::domain::types::Block;
    use chrono::Utc;
    use mockall::predicate::eq;

    const ID: &str = "00000000-0000-4000-a000-000000000001";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn make_block(id: Uuid) -> Block {
        Block {
            id,
            name: "Alpha".to_string(),
            content: "old content".to_string(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    #[test]
    fn happy_path_returns_save_write() {
        let mut blocks = MockBlockStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id())));

        let result = execute(&blocks, id(), "new content").unwrap();

        assert_eq!(result.event.block_id, id());
        assert_eq!(result.writes.len(), 1);
        assert!(matches!(&result.writes[0], VaultWrite::SaveBlock(b) if b.content == "new content"));
    }

    #[test]
    fn block_not_found_returns_error() {
        let mut blocks = MockBlockStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(|_| None);

        let result = execute(&blocks, id(), "content");
        assert!(matches!(result, Err(DomainError::BlockNotFound(_))));
    }

    #[test]
    fn heading_in_content_returns_error() {
        let mut blocks = MockBlockStore::new();

        blocks
            .expect_get()
            .with(eq(id()))
            .return_once(move |_| Some(make_block(id())));

        let result = execute(&blocks, id(), "## Heading");
        assert!(matches!(result, Err(DomainError::HeadingInContent)));
    }
}
