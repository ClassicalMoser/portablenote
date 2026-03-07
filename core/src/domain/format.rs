//! Markdown block file format.
//!
//! The system is markdown-native. Block content is markdown; this module implements
//! parsing and serialization of the canonical markdown block file format (metadata
//! comment + body). Export to other formats lives outside the domain.

use std::collections::HashMap;

use chrono::{DateTime, SecondsFormat, Utc};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use super::types::Block;

fn normalize_lf(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn normalize_nfc(s: &str) -> String {
    s.nfc().collect()
}

/// Errors encountered when parsing a block `.md` file.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("missing metadata header (expected <!-- ... --> comment)")]
    MissingMetadataHeader,

    #[error("missing required metadata field: {0}")]
    MissingField(&'static str),

    #[error("invalid UUID in '{field}': {value}")]
    InvalidUuid { field: &'static str, value: String },

    #[error("invalid timestamp in '{field}': {value}")]
    InvalidTimestamp { field: &'static str, value: String },
}

/// Extract the raw YAML metadata string and body content from a block file.
///
/// The metadata must be an HTML comment (`<!-- ... -->`) at the start of the file.
/// Returns `(metadata_str, body_content)`.
pub fn extract_metadata_header(raw: &str) -> Result<(String, String), FormatError> {
    let trimmed = raw.trim_start();

    if let Some(after_open) = trimmed.strip_prefix("<!--") {
        if let Some(close_pos) = after_open.find("-->") {
            let yaml = after_open[..close_pos].trim().to_string();
            let rest = &after_open[close_pos + 3..];
            let content = rest
                .trim_start_matches('\r')
                .trim_start_matches('\n')
                .trim_start_matches('\r')
                .trim_start_matches('\n');
            return Ok((yaml, content.to_string()));
        }
    }

    Err(FormatError::MissingMetadataHeader)
}

/// Parse simple `key: value` lines from a metadata header string.
///
/// Splits on the *first* colon only, so values containing colons
/// (e.g. ISO 8601 timestamps) are preserved intact.
pub fn parse_metadata_fields(header: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in header.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}

/// Parse a block `.md` file into a `Block`.
///
/// Expected format:
/// ```text
/// <!--
/// id: <uuid>
/// name: <human-readable name>
/// created: <iso8601>
/// modified: <iso8601>
/// -->
///
/// <content>
/// ```
pub fn parse_block_file(raw: &str) -> Result<Block, FormatError> {
    let normalized = normalize_lf(raw);
    let (header, content) = extract_metadata_header(&normalized)?;
    let fields = parse_metadata_fields(&header);

    let id_str = fields.get("id").ok_or(FormatError::MissingField("id"))?;
    let id = Uuid::parse_str(id_str).map_err(|_| FormatError::InvalidUuid {
        field: "id",
        value: id_str.clone(),
    })?;

    let name = normalize_nfc(
        fields
            .get("name")
            .ok_or(FormatError::MissingField("name"))?,
    );

    let created_str = fields
        .get("created")
        .ok_or(FormatError::MissingField("created"))?;
    let created = created_str
        .parse::<DateTime<Utc>>()
        .map_err(|_| FormatError::InvalidTimestamp {
            field: "created",
            value: created_str.clone(),
        })?;

    let modified_str = fields
        .get("modified")
        .ok_or(FormatError::MissingField("modified"))?;
    let modified = modified_str
        .parse::<DateTime<Utc>>()
        .map_err(|_| FormatError::InvalidTimestamp {
            field: "modified",
            value: modified_str.clone(),
        })?;

    Ok(Block {
        id,
        name,
        content,
        created,
        modified,
    })
}

/// Parse a block file permissively, filling defaults for missing non-ID fields.
///
/// Used during vault loading where malformed files need to be ingested for
/// validation rather than rejected outright. Fails only if the metadata header
/// is missing entirely or the `id` field is absent/invalid.
pub fn parse_block_file_permissive(raw: &str) -> Result<Block, FormatError> {
    let normalized = normalize_lf(raw);
    let (header, content) = extract_metadata_header(&normalized)?;
    let fields = parse_metadata_fields(&header);

    let id_str = fields.get("id").ok_or(FormatError::MissingField("id"))?;
    let id = Uuid::parse_str(id_str).map_err(|_| FormatError::InvalidUuid {
        field: "id",
        value: id_str.clone(),
    })?;

    Ok(Block {
        id,
        name: fields
            .get("name")
            .map(|n| normalize_nfc(n))
            .unwrap_or_default(),
        content,
        created: fields
            .get("created")
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        modified: fields
            .get("modified")
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
    })
}

/// Serialize a `Block` to the canonical block file format.
///
/// Normalizes name to NFC and content line endings to LF per spec §1.
pub fn serialize_block_file(block: &Block) -> String {
    let name = normalize_nfc(&block.name);
    let content = normalize_lf(&block.content);

    let mut out = String::new();
    out.push_str("<!--\n");
    out.push_str(&format!("id: {}\n", block.id));
    out.push_str(&format!("name: {}\n", name));
    out.push_str(&format!(
        "created: {}\n",
        block.created.to_rfc3339_opts(SecondsFormat::Secs, true)
    ));
    out.push_str(&format!(
        "modified: {}\n",
        block.modified.to_rfc3339_opts(SecondsFormat::Secs, true)
    ));
    out.push_str("-->\n");
    if !content.is_empty() {
        out.push('\n');
        out.push_str(&content);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK_ID: &str = "10000000-0000-4000-a000-000000000001";

    fn minimal_block_file() -> String {
        format!(
            "<!--\nid: {BLOCK_ID}\nname: Welcome\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\n\nThis is a minimal block.\n"
        )
    }

    fn block_file_with_refs() -> String {
        format!(
            "<!--\nid: {BLOCK_ID}\nname: Core Concepts\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\n\nSee [[Getting Started]] for context.\n\n<!-- refs -->\n[Getting Started]: uuid:20000000-0000-4000-a000-000000000002\n"
        )
    }

    // --- extract_metadata_header ---

    #[test]
    fn extract_header_splits_metadata_and_content() {
        let raw = minimal_block_file();
        let (header, content) = extract_metadata_header(&raw).unwrap();
        assert!(header.contains("id:"));
        assert!(header.contains("name: Welcome"));
        assert_eq!(content, "This is a minimal block.\n");
    }

    #[test]
    fn extract_header_missing_comment_is_error() {
        let err = extract_metadata_header("No comment here.").unwrap_err();
        assert!(matches!(err, FormatError::MissingMetadataHeader));
    }

    #[test]
    fn extract_header_unclosed_comment_is_error() {
        let err = extract_metadata_header("<!-- no close\nstuff").unwrap_err();
        assert!(matches!(err, FormatError::MissingMetadataHeader));
    }

    // --- parse_metadata_fields ---

    #[test]
    fn parse_fields_extracts_key_values() {
        let header = "id: abc-123\nname: My Block\ncreated: 2026-03-01T00:00:00Z";
        let fields = parse_metadata_fields(header);
        assert_eq!(fields.get("id").unwrap(), "abc-123");
        assert_eq!(fields.get("name").unwrap(), "My Block");
        assert_eq!(fields.get("created").unwrap(), "2026-03-01T00:00:00Z");
    }

    #[test]
    fn parse_fields_preserves_colons_in_values() {
        let header = "modified: 2026-03-01T12:34:56Z";
        let fields = parse_metadata_fields(header);
        assert_eq!(fields.get("modified").unwrap(), "2026-03-01T12:34:56Z");
    }

    #[test]
    fn parse_fields_skips_empty_keys() {
        let fields = parse_metadata_fields(": orphan value\ngood: value");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields.get("good").unwrap(), "value");
    }

    // --- parse_block_file ---

    #[test]
    fn parse_minimal_block() {
        let raw = minimal_block_file();
        let block = parse_block_file(&raw).unwrap();
        assert_eq!(block.id, Uuid::parse_str(BLOCK_ID).unwrap());
        assert_eq!(block.name, "Welcome");
        assert_eq!(block.content, "This is a minimal block.\n");
    }

    #[test]
    fn parse_block_with_footer_annotations() {
        let raw = block_file_with_refs();
        let block = parse_block_file(&raw).unwrap();
        assert_eq!(block.name, "Core Concepts");
        assert!(block.content.contains("[[Getting Started]]"));
        assert!(block.content.contains("<!-- refs -->"));
        assert!(block
            .content
            .contains("[Getting Started]: uuid:20000000-0000-4000-a000-000000000002"));
    }

    #[test]
    fn parse_missing_header_is_error() {
        let err = parse_block_file("Just content, no metadata.").unwrap_err();
        assert!(matches!(err, FormatError::MissingMetadataHeader));
    }

    #[test]
    fn parse_missing_id_is_error() {
        let raw = "<!--\nname: Test\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\n\nContent.\n";
        let err = parse_block_file(raw).unwrap_err();
        assert!(matches!(err, FormatError::MissingField("id")));
    }

    #[test]
    fn parse_missing_name_is_error() {
        let raw = format!("<!--\nid: {BLOCK_ID}\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\n\nContent.\n");
        let err = parse_block_file(&raw).unwrap_err();
        assert!(matches!(err, FormatError::MissingField("name")));
    }

    #[test]
    fn parse_invalid_uuid_is_error() {
        let raw = "<!--\nid: not-a-uuid\nname: Test\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\n\nContent.\n";
        let err = parse_block_file(raw).unwrap_err();
        assert!(matches!(err, FormatError::InvalidUuid { .. }));
    }

    #[test]
    fn parse_invalid_timestamp_is_error() {
        let raw = format!("<!--\nid: {BLOCK_ID}\nname: Test\ncreated: not-a-date\nmodified: 2026-03-01T00:00:00Z\n-->\n\nContent.\n");
        let err = parse_block_file(&raw).unwrap_err();
        assert!(matches!(err, FormatError::InvalidTimestamp { .. }));
    }

    // --- serialize_block_file ---

    #[test]
    fn serialize_produces_canonical_format() {
        let id = Uuid::parse_str(BLOCK_ID).unwrap();
        let ts = "2026-03-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let block = Block {
            id,
            name: "Welcome".to_string(),
            content: "This is a minimal block.\n".to_string(),
            created: ts,
            modified: ts,
        };

        let serialized = serialize_block_file(&block);

        assert!(serialized.starts_with("<!--\n"));
        assert!(serialized.contains(&format!("id: {BLOCK_ID}\n")));
        assert!(serialized.contains("name: Welcome\n"));
        assert!(serialized.contains("created: 2026-03-01T00:00:00Z\n"));
        assert!(serialized.contains("modified: 2026-03-01T00:00:00Z\n"));
        assert!(serialized.contains("-->\n\nThis is a minimal block.\n"));
    }

    #[test]
    fn serialize_empty_content_omits_separator() {
        let id = Uuid::parse_str(BLOCK_ID).unwrap();
        let ts = "2026-03-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let block = Block {
            id,
            name: "Empty".to_string(),
            content: String::new(),
            created: ts,
            modified: ts,
        };

        let serialized = serialize_block_file(&block);
        assert!(serialized.ends_with("-->\n"));
    }

    // --- round-trip ---

    #[test]
    fn round_trip_minimal_block() {
        let raw = minimal_block_file();
        let block = parse_block_file(&raw).unwrap();
        let reserialized = serialize_block_file(&block);
        let reparsed = parse_block_file(&reserialized).unwrap();

        assert_eq!(block.id, reparsed.id);
        assert_eq!(block.name, reparsed.name);
        assert_eq!(block.content, reparsed.content);
        assert_eq!(block.created, reparsed.created);
        assert_eq!(block.modified, reparsed.modified);
    }

    // --- parse_block_file_permissive ---

    #[test]
    fn permissive_fills_defaults_for_missing_name() {
        let raw = format!(
            "<!--\nid: {BLOCK_ID}\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\n\nContent without a name.\n"
        );
        let block = parse_block_file_permissive(&raw).unwrap();
        assert_eq!(block.id, Uuid::parse_str(BLOCK_ID).unwrap());
        assert!(block.name.is_empty());
    }

    #[test]
    fn permissive_fills_defaults_for_missing_timestamps() {
        let raw = format!(
            "<!--\nid: {BLOCK_ID}\nname: Test\n-->\n\nContent.\n"
        );
        let block = parse_block_file_permissive(&raw).unwrap();
        assert_eq!(block.name, "Test");
        assert_eq!(block.created, DateTime::<Utc>::default());
        assert_eq!(block.modified, DateTime::<Utc>::default());
    }

    #[test]
    fn permissive_still_requires_valid_id() {
        let raw = "<!--\nname: Test\n-->\n\nContent.\n";
        let err = parse_block_file_permissive(raw).unwrap_err();
        assert!(matches!(err, FormatError::MissingField("id")));
    }

    #[test]
    fn permissive_still_requires_metadata_header() {
        let err = parse_block_file_permissive("Just content.").unwrap_err();
        assert!(matches!(err, FormatError::MissingMetadataHeader));
    }

    #[test]
    fn serialize_normalizes_crlf_to_lf() {
        let id = Uuid::parse_str(BLOCK_ID).unwrap();
        let ts = "2026-03-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let block = Block {
            id,
            name: "Test".to_string(),
            content: "line1\r\nline2\r\n".to_string(),
            created: ts,
            modified: ts,
        };

        let serialized = serialize_block_file(&block);
        assert!(
            !serialized.contains("\r\n"),
            "serialized output must not contain CRLF"
        );
        assert!(serialized.contains("line1\nline2\n"));
    }

    #[test]
    fn serialize_normalizes_name_to_nfc() {
        let id = Uuid::parse_str(BLOCK_ID).unwrap();
        let ts = "2026-03-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        // NFD: e + combining acute
        let block = Block {
            id,
            name: "cafe\u{0301}".to_string(),
            content: String::new(),
            created: ts,
            modified: ts,
        };

        let serialized = serialize_block_file(&block);
        // NFC: é = U+00E9
        assert!(
            serialized.contains("name: caf\u{00E9}\n"),
            "name must be NFC-normalized in serialized output"
        );
    }

    #[test]
    fn parse_normalizes_crlf_content_to_lf() {
        let raw = format!(
            "<!--\nid: {BLOCK_ID}\nname: Test\ncreated: 2026-03-01T00:00:00Z\nmodified: 2026-03-01T00:00:00Z\n-->\r\n\r\nline1\r\nline2\r\n"
        );
        let block = parse_block_file(&raw).unwrap();
        assert!(
            !block.content.contains('\r'),
            "parsed content must not contain CR"
        );
    }

    // --- round-trip ---

    #[test]
    fn round_trip_block_with_refs() {
        let raw = block_file_with_refs();
        let block = parse_block_file(&raw).unwrap();
        let reserialized = serialize_block_file(&block);
        let reparsed = parse_block_file(&reserialized).unwrap();

        assert_eq!(block.id, reparsed.id);
        assert_eq!(block.name, reparsed.name);
        assert_eq!(block.content, reparsed.content);
        assert_eq!(block.created, reparsed.created);
        assert_eq!(block.modified, reparsed.modified);
    }
}
