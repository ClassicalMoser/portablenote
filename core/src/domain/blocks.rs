use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::content;
use super::error::DomainError;
use super::types::Block;

/// Reserved for CommonMark link syntax `[text](url)` (e.g. block-reference links).
fn name_has_reserved_characters(name: &str) -> bool {
    name.contains('[') || name.contains(']')
}

/// Percent is disallowed in block names (ambiguous with percent-encoding in filenames).
pub fn name_contains_percent(name: &str) -> bool {
    name.contains('%')
}

/// Create a new block, enforcing the no-heading and non-empty-name invariants.
/// Time is passed in so the domain stays pure and testable.
pub fn create(
    id: Uuid,
    name: &str,
    content_str: &str,
    now: DateTime<Utc>,
) -> Result<Block, DomainError> {
    if name.is_empty() {
        return Err(DomainError::EmptyName);
    }
    if name_has_reserved_characters(name) {
        return Err(DomainError::NameContainsReservedCharacters);
    }
    if name_contains_percent(name) {
        return Err(DomainError::NameContainsPercent);
    }
    if content::find_heading_outside_fence(content_str).is_some() {
        return Err(DomainError::HeadingInContent);
    }
    Ok(Block {
        id,
        name: name.to_string(),
        content: content_str.to_string(),
        created: now,
        modified: now,
    })
}

/// Apply a rename to a block, updating `modified`. Rejects empty or invalid names.
pub fn apply_rename(
    mut block: Block,
    new_name: &str,
    now: DateTime<Utc>,
) -> Result<Block, DomainError> {
    if new_name.is_empty() {
        return Err(DomainError::EmptyName);
    }
    if name_has_reserved_characters(new_name) {
        return Err(DomainError::NameContainsReservedCharacters);
    }
    if name_contains_percent(new_name) {
        return Err(DomainError::NameContainsPercent);
    }
    block.name = new_name.to_string();
    block.modified = now;
    Ok(block)
}

/// Replace a block's content, enforcing the no-heading invariant.
pub fn apply_content(
    mut block: Block,
    content_str: &str,
    now: DateTime<Utc>,
) -> Result<Block, DomainError> {
    if content::find_heading_outside_fence(content_str).is_some() {
        return Err(DomainError::HeadingInContent);
    }
    block.content = content_str.to_string();
    block.modified = now;
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "00000000-0000-4000-a000-000000000001";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn create_happy_path() {
        let block = create(id(), "Alpha", "Some content.", now()).unwrap();
        assert_eq!(block.id, id());
        assert_eq!(block.name, "Alpha");
        assert_eq!(block.content, "Some content.");
        assert_eq!(block.created, block.modified);
    }

    #[test]
    fn create_empty_name_is_error() {
        assert!(matches!(create(id(), "", "content", now()), Err(DomainError::EmptyName)));
    }

    #[test]
    fn create_name_with_bracket_is_error() {
        assert!(matches!(
            create(id(), "Block]Name", "content", now()),
            Err(DomainError::NameContainsReservedCharacters)
        ));
        assert!(matches!(
            create(id(), "Block[Name", "content", now()),
            Err(DomainError::NameContainsReservedCharacters)
        ));
    }

    #[test]
    fn create_name_with_percent_is_error() {
        assert!(matches!(
            create(id(), "100%", "content", now()),
            Err(DomainError::NameContainsPercent)
        ));
        assert!(matches!(
            create(id(), "Half % complete", "content", now()),
            Err(DomainError::NameContainsPercent)
        ));
    }

    #[test]
    fn create_heading_in_content_is_error() {
        assert!(matches!(
            create(id(), "Alpha", "## Heading", now()),
            Err(DomainError::HeadingInContent)
        ));
    }

    #[test]
    fn create_heading_inside_fence_is_ok() {
        assert!(create(id(), "Alpha", "```\n## Not a heading\n```", now()).is_ok());
    }

    #[test]
    fn apply_rename_updates_name_and_modified() {
        let block = create(id(), "Alpha", "content", now()).unwrap();
        let renamed = apply_rename(block, "Beta", now()).unwrap();
        assert_eq!(renamed.name, "Beta");
    }

    #[test]
    fn apply_rename_empty_name_is_error() {
        let block = create(id(), "Alpha", "content", now()).unwrap();
        assert!(matches!(apply_rename(block, "", now()), Err(DomainError::EmptyName)));
    }

    #[test]
    fn apply_rename_reserved_characters_is_error() {
        let block = create(id(), "Alpha", "content", now()).unwrap();
        assert!(matches!(
            apply_rename(block.clone(), "New[Name", now()),
            Err(DomainError::NameContainsReservedCharacters)
        ));
        assert!(matches!(
            apply_rename(block, "New]Name", now()),
            Err(DomainError::NameContainsReservedCharacters)
        ));
    }

    #[test]
    fn apply_rename_percent_is_error() {
        let block = create(id(), "Alpha", "content", now()).unwrap();
        assert!(matches!(
            apply_rename(block, "Done 100%", now()),
            Err(DomainError::NameContainsPercent)
        ));
    }

    #[test]
    fn apply_content_updates_content() {
        let block = create(id(), "Alpha", "old", now()).unwrap();
        let updated = apply_content(block, "new content", now()).unwrap();
        assert_eq!(updated.content, "new content");
    }

    #[test]
    fn apply_content_heading_is_error() {
        let block = create(id(), "Alpha", "content", now()).unwrap();
        assert!(matches!(
            apply_content(block, "# Heading", now()),
            Err(DomainError::HeadingInContent)
        ));
    }

}
