use std::collections::HashMap;

use uuid::Uuid;

/// Find the first heading (ATX or setext) outside a fenced code block.
/// Returns `(level, heading_text)` if found.
///
/// Detects both ATX headings (`# Heading`) and setext headings (paragraph
/// text followed immediately by a line of `=` or `-` characters).
///
/// Respects CommonMark indentation rules: lines with 4+ leading spaces are
/// indented code blocks, not structural elements.
pub fn find_heading_outside_fence(content: &str) -> Option<(u8, String)> {
    let mut in_fence = false;
    let mut prev_line: Option<&str> = None;

    for line in content.lines() {
        let indent = leading_spaces(line);
        let stripped = line[indent..].trim_end();

        if in_fence {
            if indent <= 3 && is_fence_marker(stripped) {
                in_fence = false;
            }
            prev_line = None;
            continue;
        }

        // 4+ spaces: indented code block — not structural
        if indent >= 4 {
            prev_line = None;
            continue;
        }

        if is_fence_marker(stripped) {
            in_fence = true;
            prev_line = None;
            continue;
        }

        // ATX heading: # through ######
        if let Some(level_and_text) = detect_atx_heading(stripped) {
            return Some(level_and_text);
        }

        // Setext heading: paragraph text followed by a line of = or -
        if let Some(prev) = prev_line {
            if is_setext_underline(stripped) {
                let level = if stripped.starts_with('=') { 1 } else { 2 };
                return Some((level, prev.trim().to_string()));
            }
        }

        if stripped.is_empty() {
            prev_line = None;
        } else {
            prev_line = Some(stripped);
        }
    }

    None
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').count()
}

fn is_fence_marker(line: &str) -> bool {
    (line.starts_with("```") && line.trim_start_matches('`').find('`').is_none())
        || (line.starts_with("~~~") && line.trim_start_matches('~').find('~').is_none())
}

fn detect_atx_heading(line: &str) -> Option<(u8, String)> {
    let stripped = line.strip_prefix('#')?;
    let mut level: u8 = 1;
    let mut rest = stripped;

    while let Some(s) = rest.strip_prefix('#') {
        level += 1;
        rest = s;
        if level > 6 {
            return None;
        }
    }

    let s = rest.strip_prefix(' ')?;
    let text = s.trim().to_string();
    if text.is_empty() {
        return None;
    }
    Some((level, text))
}

/// A setext heading underline is a line consisting entirely of `=` or `-`
/// characters (at least one).
fn is_setext_underline(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let marker = line.as_bytes()[0];
    (marker == b'=' || marker == b'-') && line.bytes().all(|b| b == marker)
}

/// Extract all `[[Name]]` inline references from block content.
pub fn extract_inline_refs(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = content;

    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let name = after[..end].trim();
            if !name.is_empty() {
                refs.push(name.to_string());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }

    refs
}

/// Extract footer annotations from `[Name]: uuid:<uuid>` lines after `<!-- refs -->`.
pub fn extract_footer_annotations(content: &str) -> HashMap<String, Uuid> {
    let mut map = HashMap::new();

    let Some(refs_pos) = content.find("<!-- refs -->") else {
        return map;
    };

    let footer = &content[refs_pos..];

    for line in footer.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }
        if let Some(close) = line.find("]: uuid:") {
            let name = &line[1..close];
            let uuid_str = &line[close + 8..];
            if let Ok(uuid) = Uuid::parse_str(uuid_str.trim()) {
                map.insert(name.to_string(), uuid);
            }
        }
    }

    map
}

/// Replace all `[[old_name]]` inline references with `[[new_name]]` and rename
/// the corresponding footer annotation prefix `[old_name]: uuid:` to
/// `[new_name]: uuid:`. Returns `(updated_content, count)` where `count` is
/// the number of `[[old_name]]` occurrences replaced (footer rename is not
/// counted separately — it tracks the inline ref).
pub fn rename_reference(content: &str, old_name: &str, new_name: &str) -> (String, usize) {
    let old_inline = format!("[[{old_name}]]");
    let new_inline = format!("[[{new_name}]]");

    let count = content.matches(old_inline.as_str()).count();
    let mut result = if count > 0 {
        content.replace(&old_inline, &new_inline)
    } else {
        content.to_string()
    };

    let old_footer_prefix = format!("[{old_name}]: uuid:");
    let new_footer_prefix = format!("[{new_name}]: uuid:");
    if result.contains(&old_footer_prefix) {
        result = result.replace(&old_footer_prefix, &new_footer_prefix);
    }

    (result, count)
}

/// Revert `[[name]]` to plain text `name` and remove the footer annotation
/// `[name]: uuid:<target_uuid>`. Returns `(updated_content, count)` where
/// `count` is the number of inline ref occurrences reverted.
pub fn revert_reference(content: &str, name: &str, target_uuid: Uuid) -> (String, usize) {
    let inline = format!("[[{name}]]");
    let count = content.matches(inline.as_str()).count();

    let result = if count > 0 {
        content.replace(&inline, name)
    } else {
        content.to_string()
    };

    let footer_line = format!("[{name}]: uuid:{target_uuid}");
    let has_trailing_newline = result.ends_with('\n');

    let new_result: String = result
        .lines()
        .filter(|line| line.trim() != footer_line)
        .collect::<Vec<_>>()
        .join("\n");

    let result = if has_trailing_newline {
        new_result + "\n"
    } else {
        new_result
    };

    (result, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_detected_outside_fence() {
        let content = "Some text.\n\n## Bad Heading\n\nMore text.";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((2, "Bad Heading".to_string()))
        );
    }

    #[test]
    fn heading_inside_fence_ignored() {
        let content = "Text.\n\n```\n## Not a heading\n```\n\nMore text.";
        assert!(find_heading_outside_fence(content).is_none());
    }

    #[test]
    fn no_heading_returns_none() {
        assert!(find_heading_outside_fence("Plain text.").is_none());
    }

    // --- setext headings ---

    #[test]
    fn setext_h1_with_equals() {
        let content = "Some text.\n\nBad Heading\n===\n\nMore text.";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((1, "Bad Heading".to_string()))
        );
    }

    #[test]
    fn setext_h2_with_dashes() {
        let content = "Some text.\n\nBad Heading\n---\n\nMore text.";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((2, "Bad Heading".to_string()))
        );
    }

    #[test]
    fn setext_single_dash_is_heading() {
        let content = "Heading\n-";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((2, "Heading".to_string()))
        );
    }

    #[test]
    fn setext_single_equals_is_heading() {
        let content = "Heading\n=";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((1, "Heading".to_string()))
        );
    }

    #[test]
    fn setext_inside_fence_ignored() {
        let content = "Text.\n\n```\nNot a heading\n===\n```\n\nMore text.";
        assert!(find_heading_outside_fence(content).is_none());
    }

    #[test]
    fn dashes_after_blank_line_not_heading() {
        let content = "Some text.\n\n---\n\nMore text.";
        assert!(find_heading_outside_fence(content).is_none());
    }

    #[test]
    fn equals_after_blank_line_not_heading() {
        let content = "Some text.\n\n===\n\nMore text.";
        assert!(find_heading_outside_fence(content).is_none());
    }

    #[test]
    fn mixed_underline_not_setext() {
        let content = "Not a heading\n=-=\n";
        assert!(find_heading_outside_fence(content).is_none());
    }

    // --- indentation ---

    #[test]
    fn atx_heading_with_3_spaces_is_heading() {
        let content = "   ## Indented Heading";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((2, "Indented Heading".to_string()))
        );
    }

    #[test]
    fn atx_heading_with_4_spaces_is_code() {
        let content = "    ## Not a heading";
        assert!(find_heading_outside_fence(content).is_none());
    }

    #[test]
    fn setext_underline_with_4_spaces_is_code() {
        let content = "Paragraph\n    ===";
        assert!(find_heading_outside_fence(content).is_none());
    }

    #[test]
    fn fence_with_4_spaces_does_not_open() {
        let content = "    ```\n## Real Heading\n    ```";
        assert_eq!(
            find_heading_outside_fence(content),
            Some((2, "Real Heading".to_string()))
        );
    }

    #[test]
    fn extract_refs_finds_both() {
        let refs = extract_inline_refs("See [[Getting Started]] and [[Advanced Topics]].");
        assert_eq!(refs, vec!["Getting Started", "Advanced Topics"]);
    }

    #[test]
    fn extract_refs_empty() {
        assert!(extract_inline_refs("No refs here.").is_empty());
    }

    #[test]
    fn extract_annotations_finds_entries() {
        let content = "Body.\n\n<!-- refs -->\n[Alpha]: uuid:00000000-0000-4000-a000-000000000001\n[Beta]: uuid:00000000-0000-4000-a000-000000000002\n";
        let map = extract_footer_annotations(content);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("Alpha"));
        assert!(map.contains_key("Beta"));
    }

    #[test]
    fn extract_annotations_no_marker_empty() {
        assert!(extract_footer_annotations("No marker here.").is_empty());
    }

    #[test]
    fn rename_updates_inline_ref() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let content = format!("See [[Beta]] here.\n\n<!-- refs -->\n[Beta]: uuid:{id}\n");
        let (result, count) = rename_reference(&content, "Beta", "Gamma");
        assert_eq!(count, 1);
        assert!(result.contains("[[Gamma]]"));
        assert!(!result.contains("[[Beta]]"));
        assert!(result.contains(&format!("[Gamma]: uuid:{id}")));
        assert!(!result.contains(&format!("[Beta]: uuid:{id}")));
    }

    #[test]
    fn rename_no_match_is_noop() {
        let content = "No refs here.";
        let (result, count) = rename_reference(content, "Beta", "Gamma");
        assert_eq!(count, 0);
        assert_eq!(result, content);
    }

    #[test]
    fn rename_multiple_occurrences() {
        let content = "[[Beta]] and also [[Beta]].";
        let (result, count) = rename_reference(content, "Beta", "Gamma");
        assert_eq!(count, 2);
        assert_eq!(result, "[[Gamma]] and also [[Gamma]].");
    }

    #[test]
    fn revert_removes_link_brackets_and_footer() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let content = format!("See [[Beta]] here.\n\n<!-- refs -->\n[Beta]: uuid:{id}\n");
        let (result, count) = revert_reference(&content, "Beta", id);
        assert_eq!(count, 1);
        assert!(result.contains("See Beta here."));
        assert!(!result.contains("[[Beta]]"));
        assert!(!result.contains(&format!("[Beta]: uuid:{id}")));
    }

    #[test]
    fn revert_preserves_trailing_newline() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let content = format!("Text.\n\n<!-- refs -->\n[Beta]: uuid:{id}\n");
        let (result, _) = revert_reference(&content, "Beta", id);
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn revert_no_match_is_noop() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let content = "No refs here.";
        let (result, count) = revert_reference(content, "Beta", id);
        assert_eq!(count, 0);
        assert_eq!(result, content);
    }
}
