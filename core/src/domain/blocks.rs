use chrono::Utc;
use uuid::Uuid;

use super::content;
use super::error::DomainError;
use super::types::Block;

/// Create a new block, enforcing the no-heading and non-empty-name invariants.
pub fn create(id: Uuid, name: &str, content_str: &str) -> Result<Block, DomainError> {
    if name.is_empty() {
        return Err(DomainError::EmptyName);
    }
    if content::find_heading_outside_fence(content_str).is_some() {
        return Err(DomainError::HeadingInContent);
    }
    let now = Utc::now();
    Ok(Block {
        id,
        name: name.to_string(),
        content: content_str.to_string(),
        created: now,
        modified: now,
    })
}

/// Apply a rename to a block, updating `modified`. Rejects empty names.
pub fn apply_rename(mut block: Block, new_name: &str) -> Result<Block, DomainError> {
    if new_name.is_empty() {
        return Err(DomainError::EmptyName);
    }
    block.name = new_name.to_string();
    block.modified = Utc::now();
    Ok(block)
}

/// Replace a block's content, enforcing the no-heading invariant.
pub fn apply_content(mut block: Block, content_str: &str) -> Result<Block, DomainError> {
    if content::find_heading_outside_fence(content_str).is_some() {
        return Err(DomainError::HeadingInContent);
    }
    block.content = content_str.to_string();
    block.modified = Utc::now();
    Ok(block)
}

/// Propagate a rename through a set of referencing blocks.
/// Returns `(updated_blocks, total_inline_refs_updated)`.
pub fn propagate_rename(
    blocks: Vec<Block>,
    old_name: &str,
    new_name: &str,
) -> (Vec<Block>, usize) {
    let mut total = 0;
    let updated = blocks
        .into_iter()
        .map(|mut b| {
            let (new_content, count) = content::rename_reference(&b.content, old_name, new_name);
            if count > 0 {
                b.content = new_content;
                b.modified = Utc::now();
                total += count;
            }
            b
        })
        .collect();
    (updated, total)
}

/// Revert inline references to a deleted block back to plain text.
/// Returns `(updated_blocks, total_inline_refs_reverted)`.
pub fn revert_refs(blocks: Vec<Block>, name: &str, target_uuid: Uuid) -> (Vec<Block>, usize) {
    let mut total = 0;
    let updated = blocks
        .into_iter()
        .map(|mut b| {
            let (new_content, count) = content::revert_reference(&b.content, name, target_uuid);
            if count > 0 {
                b.content = new_content;
                b.modified = Utc::now();
                total += count;
            }
            b
        })
        .collect();
    (updated, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "00000000-0000-4000-a000-000000000001";
    const OTHER: &str = "00000000-0000-4000-a000-000000000002";

    fn id() -> Uuid {
        Uuid::parse_str(ID).unwrap()
    }
    fn other() -> Uuid {
        Uuid::parse_str(OTHER).unwrap()
    }

    #[test]
    fn create_happy_path() {
        let block = create(id(), "Alpha", "Some content.").unwrap();
        assert_eq!(block.id, id());
        assert_eq!(block.name, "Alpha");
        assert_eq!(block.content, "Some content.");
        assert_eq!(block.created, block.modified);
    }

    #[test]
    fn create_empty_name_is_error() {
        assert!(matches!(create(id(), "", "content"), Err(DomainError::EmptyName)));
    }

    #[test]
    fn create_heading_in_content_is_error() {
        assert!(matches!(
            create(id(), "Alpha", "## Heading"),
            Err(DomainError::HeadingInContent)
        ));
    }

    #[test]
    fn create_heading_inside_fence_is_ok() {
        assert!(create(id(), "Alpha", "```\n## Not a heading\n```").is_ok());
    }

    #[test]
    fn apply_rename_updates_name_and_modified() {
        let block = create(id(), "Alpha", "content").unwrap();
        let renamed = apply_rename(block, "Beta").unwrap();
        assert_eq!(renamed.name, "Beta");
    }

    #[test]
    fn apply_rename_empty_name_is_error() {
        let block = create(id(), "Alpha", "content").unwrap();
        assert!(matches!(apply_rename(block, ""), Err(DomainError::EmptyName)));
    }

    #[test]
    fn apply_content_updates_content() {
        let block = create(id(), "Alpha", "old").unwrap();
        let updated = apply_content(block, "new content").unwrap();
        assert_eq!(updated.content, "new content");
    }

    #[test]
    fn apply_content_heading_is_error() {
        let block = create(id(), "Alpha", "content").unwrap();
        assert!(matches!(
            apply_content(block, "# Heading"),
            Err(DomainError::HeadingInContent)
        ));
    }

    #[test]
    fn propagate_rename_updates_refs() {
        let b1 = create(id(), "Referrer", "See [[Alpha]].\n\n<!-- refs -->\n[Alpha]: uuid:00000000-0000-4000-a000-000000000002\n").unwrap();
        let (updated, count) = propagate_rename(vec![b1], "Alpha", "Beta");
        assert_eq!(count, 1);
        assert!(updated[0].content.contains("[[Beta]]"));
    }

    #[test]
    fn propagate_rename_no_match_unchanged() {
        let b = create(id(), "NoRef", "plain text").unwrap();
        let (updated, count) = propagate_rename(vec![b], "Alpha", "Beta");
        assert_eq!(count, 0);
        assert_eq!(updated[0].content, "plain text");
    }

    #[test]
    fn revert_refs_removes_link_syntax() {
        let content = format!(
            "See [[Alpha]] here.\n\n<!-- refs -->\n[Alpha]: uuid:{OTHER}\n"
        );
        let b = create(id(), "Referrer", &content).unwrap();
        let (updated, count) = revert_refs(vec![b], "Alpha", other());
        assert_eq!(count, 1);
        assert!(updated[0].content.contains("See Alpha here."));
        assert!(!updated[0].content.contains("[[Alpha]]"));
    }

    #[test]
    fn revert_refs_no_match_unchanged() {
        let b = create(id(), "NoRef", "plain text").unwrap();
        let (updated, count) = revert_refs(vec![b], "Alpha", other());
        assert_eq!(count, 0);
        assert_eq!(updated[0].content, "plain text");
    }
}
