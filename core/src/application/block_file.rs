//! Block file format: extract and update block-reference links.
//!
//! Application-owned. Block references use CommonMark inline link syntax
//! `[display text](block:uuid)` only. No footer; no reference-style.

use uuid::Uuid;

/// Scheme prefix for block-reference links.
const BLOCK_SCHEME: &str = "block:";

/// Extract all `[text](block:uuid)` links from content, in order.
/// Returns (display_text, target_uuid) for each link. Multiple links to the
/// same block are all returned.
pub fn extract_block_refs(content: &str) -> Vec<(String, Uuid)> {
    let mut refs = Vec::new();
    let mut i = 0;
    let bytes = content.as_bytes();

    while i < bytes.len() {
        // Find next "(block:"
        let Some(open_paren) = content[i..].find("(block:") else {
            break;
        };
        let paren_start = i + open_paren;
        let scheme_start = paren_start + 1; // after '('
        let uuid_start = scheme_start + BLOCK_SCHEME.len();

        if uuid_start + 36 > bytes.len() {
            break;
        }

        let uuid_slice = &content[uuid_start..uuid_start + 36];
        let Ok(uuid) = Uuid::parse_str(uuid_slice) else {
            i = uuid_start + 1;
            continue;
        };

        if bytes.get(uuid_start + 36) != Some(&b')') {
            i = uuid_start + 1;
            continue;
        }

        // The ']' that closes the link text is immediately before '(' in "](block:"
        let close_bracket = paren_start.saturating_sub(1);
        if let Some(open_bracket) = content[..=close_bracket].rfind('[') {
            let display = content[open_bracket + 1..close_bracket].to_string();
            refs.push((display, uuid));
        }

        i = uuid_start + 37; // past ")
    }

    refs
}

/// Replace every `[old](block:target_uuid)` in content with `[new_name](block:target_uuid)`.
/// Returns (updated_content, count).
pub fn rename_refs_in_content(
    content: &str,
    target_uuid: Uuid,
    new_name: &str,
) -> (String, usize) {
    let needle = format!("]({BLOCK_SCHEME}{target_uuid})");
    let mut count = 0;
    let mut result = content.to_string();
    let mut search_start = 0;

    while let Some(close) = result[search_start..].find(&needle) {
        let needle_start = search_start + close;
        let link_end = needle_start + needle.len();
        // Find the preceding "["
        let content_start = result[..needle_start].rfind('[').unwrap_or(0);
        let new_link = format!("[{new_name}]({BLOCK_SCHEME}{target_uuid})");
        result.replace_range(content_start..link_end, &new_link);
        count += 1;
        search_start = content_start + new_link.len();
    }

    (result, count)
}

/// Revert every `[text](block:target_uuid)` in content to plain `text`.
/// Returns (updated_content, count).
pub fn revert_refs_in_content(content: &str, target_uuid: Uuid) -> (String, usize) {
    let needle = format!("]({BLOCK_SCHEME}{target_uuid})");
    let mut count = 0;
    let mut result = content.to_string();
    let mut search_start = 0;

    while let Some(close) = result[search_start..].find(&needle) {
        let needle_start = search_start + close;
        let link_end = needle_start + needle.len();
        let content_start = result[..needle_start].rfind('[').unwrap_or(0);
        let display_text = result[content_start + 1..needle_start].to_string();
        let len = display_text.len();
        result.replace_range(content_start..link_end, &display_text);
        count += 1;
        search_start = content_start + len;
    }

    (result, count)
}

/// True if content contains at least one link to the given block.
pub fn content_references_block(content: &str, target_uuid: Uuid) -> bool {
    let needle = format!("]({BLOCK_SCHEME}{target_uuid})");
    content.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID1: &str = "00000000-0000-4000-a000-000000000001";
    const UUID2: &str = "00000000-0000-4000-a000-000000000002";

    #[test]
    fn extract_finds_inline_block_refs() {
        let content = format!(
            "See [Getting Started](block:{UUID1}) and [Advanced](block:{UUID2})."
        );
        let refs = extract_block_refs(&content);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].0, "Getting Started");
        assert_eq!(refs[0].1, Uuid::parse_str(UUID1).unwrap());
        assert_eq!(refs[1].0, "Advanced");
        assert_eq!(refs[1].1, Uuid::parse_str(UUID2).unwrap());
    }

    #[test]
    fn extract_multiple_refs_to_same_block() {
        let content = format!(
            "See [Alpha](block:{UUID1}) here and [Alpha](block:{UUID1}) again."
        );
        let refs = extract_block_refs(&content);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].1, Uuid::parse_str(UUID1).unwrap());
        assert_eq!(refs[1].1, Uuid::parse_str(UUID1).unwrap());
    }

    #[test]
    fn extract_empty() {
        assert!(extract_block_refs("No links here.").is_empty());
    }

    #[test]
    fn rename_updates_all_occurrences() {
        let id = Uuid::parse_str(UUID1).unwrap();
        let content = format!("See [Alpha](block:{UUID1}) and [Alpha](block:{UUID1}).");
        let (out, count) = rename_refs_in_content(&content, id, "Beta");
        assert_eq!(count, 2);
        assert!(out.contains("[Beta](block:"));
        assert!(!out.contains("[Alpha](block:"));
    }

    #[test]
    fn revert_removes_link_syntax() {
        let id = Uuid::parse_str(UUID1).unwrap();
        let content = format!("See [Alpha](block:{UUID1}) here.");
        let (out, count) = revert_refs_in_content(&content, id);
        assert_eq!(count, 1);
        assert_eq!(out, "See Alpha here.");
    }

    #[test]
    fn content_references_block_true() {
        let id = Uuid::parse_str(UUID1).unwrap();
        let content = format!("Link [x](block:{UUID1})");
        assert!(content_references_block(&content, id));
    }

    #[test]
    fn content_references_block_false() {
        let id = Uuid::parse_str(UUID1).unwrap();
        assert!(!content_references_block("No link.", id));
    }
}
