//! Assertion evaluation engine for mutation compliance tests.
//!
//! Each assertion variant from the scenario JSON maps to a check against
//! the post-mutation `VaultStores` state (and optionally the command outcome
//! for event assertions).

#![allow(dead_code)] // shared test infra — not every binary uses every function

use chrono::DateTime;
use uuid::Uuid;

use portablenote_core::application::ports::{BlockStore, DocumentStore, GraphStore, NameIndex};
use portablenote_core::domain::content;

use super::harness::CommandOutcome;
use super::in_memory::VaultStores;
use super::scenario::Assertion;

fn parse_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_else(|_| panic!("invalid UUID in assertion: {s}"))
}

/// Evaluate a single assertion against the current store state and command outcome.
/// Panics with a descriptive message on failure.
pub fn evaluate(assertion: &Assertion, stores: &VaultStores, outcome: &CommandOutcome) {
    match assertion {
        Assertion::BlockExists { block_id } => {
            let id = parse_uuid(block_id);
            assert!(
                stores.blocks.get(id).is_some(),
                "expected block {block_id} to exist"
            );
        }

        Assertion::BlockNotExists { block_id } => {
            let id = parse_uuid(block_id);
            assert!(
                stores.blocks.get(id).is_none(),
                "expected block {block_id} NOT to exist"
            );
        }

        Assertion::BlockContentIs { block_id, content: expected } => {
            let id = parse_uuid(block_id);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for content_is check"));
            assert_eq!(
                block.content.trim(),
                expected.trim(),
                "block {block_id} content mismatch"
            );
        }

        Assertion::BlockContentContains { block_id, text } => {
            let id = parse_uuid(block_id);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for content_contains check"));
            assert!(
                block.content.contains(text.as_str()),
                "expected block {block_id} content to contain '{text}', got: {}",
                block.content
            );
        }

        Assertion::BlockContentNotContains { block_id, text } => {
            let id = parse_uuid(block_id);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for content_not_contains check"));
            assert!(
                !block.content.contains(text.as_str()),
                "expected block {block_id} content NOT to contain '{text}', got: {}",
                block.content
            );
        }

        Assertion::BlockFooterNotContains { block_id, name } => {
            let id = parse_uuid(block_id);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for footer check"));
            let annotations = content::extract_footer_annotations(&block.content);
            assert!(
                !annotations.contains_key(name.as_str()),
                "expected footer of block {block_id} NOT to contain annotation for '{name}'"
            );
        }

        Assertion::BlockModifiedAfter { block_id, after } => {
            let id = parse_uuid(block_id);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for modified_after check"));
            let threshold = after.parse::<DateTime<chrono::Utc>>()
                .unwrap_or_else(|e| panic!("invalid timestamp '{after}': {e}"));
            assert!(
                block.modified > threshold,
                "expected block {block_id} modified ({}) to be after {after}",
                block.modified
            );
        }

        Assertion::NameIndexMissing { name } => {
            assert!(
                stores.names.resolve(name).is_none(),
                "expected name '{name}' to be absent from name index"
            );
        }

        Assertion::NameIndexContains { name, block_id } => {
            let expected_id = parse_uuid(block_id);
            let resolved = stores.names.resolve(name);
            assert_eq!(
                resolved,
                Some(expected_id),
                "expected name '{name}' to resolve to {block_id}"
            );
        }

        Assertion::BlockCount { count } => {
            let actual = stores.blocks.list().len();
            assert_eq!(
                actual, *count,
                "expected {count} blocks, found {actual}"
            );
        }

        Assertion::BlockNameIs { block_id, name } => {
            let id = parse_uuid(block_id);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for name_is check"));
            assert_eq!(
                block.name, *name,
                "block {block_id} name mismatch"
            );
        }

        Assertion::BlockFooterMaps { block_id, name, target_uuid } => {
            let id = parse_uuid(block_id);
            let expected_target = parse_uuid(target_uuid);
            let block = stores.blocks.get(id)
                .unwrap_or_else(|| panic!("block {block_id} not found for footer_maps check"));
            let annotations = content::extract_footer_annotations(&block.content);
            let actual = annotations.get(name.as_str());
            assert_eq!(
                actual,
                Some(&expected_target),
                "expected footer of block {block_id} to map '{name}' -> {target_uuid}"
            );
        }

        Assertion::EdgeExists { edge_id } => {
            let id = parse_uuid(edge_id);
            assert!(
                stores.graph.get_edge(id).is_some(),
                "expected edge {edge_id} to exist"
            );
        }

        Assertion::EdgeNotExists { edge_id } => {
            let id = parse_uuid(edge_id);
            assert!(
                stores.graph.get_edge(id).is_none(),
                "expected edge {edge_id} NOT to exist"
            );
        }

        Assertion::EdgeCount { count } => {
            let actual = stores.graph.edges.len();
            assert_eq!(
                actual, *count,
                "expected {count} edges, found {actual}"
            );
        }

        Assertion::DocumentExists { document_id } => {
            let id = parse_uuid(document_id);
            assert!(
                stores.documents.get(id).is_some(),
                "expected document {document_id} to exist"
            );
        }

        Assertion::DocumentNotExists { document_id } => {
            let id = parse_uuid(document_id);
            assert!(
                stores.documents.get(id).is_none(),
                "expected document {document_id} NOT to exist"
            );
        }

        Assertion::DocumentRootIs { document_id, root } => {
            let id = parse_uuid(document_id);
            let expected_root = parse_uuid(root);
            let doc = stores.documents.get(id)
                .unwrap_or_else(|| panic!("document {document_id} not found for root check"));
            assert_eq!(
                doc.root, expected_root,
                "document {document_id} root mismatch"
            );
        }

        Assertion::DocumentSectionCount { document_id, count } => {
            let id = parse_uuid(document_id);
            let doc = stores.documents.get(id)
                .unwrap_or_else(|| panic!("document {document_id} not found for section_count check"));
            assert_eq!(
                doc.sections.len(), *count,
                "document {document_id} section count mismatch"
            );
        }

        Assertion::DocumentSectionAt { document_id, index, block_id } => {
            let id = parse_uuid(document_id);
            let expected_block = parse_uuid(block_id);
            let doc = stores.documents.get(id)
                .unwrap_or_else(|| panic!("document {document_id} not found for section_at check"));
            let section = doc.sections.get(*index)
                .unwrap_or_else(|| panic!("document {document_id} has no section at index {index}"));
            assert_eq!(
                section.block, expected_block,
                "document {document_id} section[{index}] block mismatch"
            );
        }

        Assertion::DocumentSubsectionCount { document_id, section_block_id, count } => {
            let id = parse_uuid(document_id);
            let sec_block = parse_uuid(section_block_id);
            let doc = stores.documents.get(id)
                .unwrap_or_else(|| panic!("document {document_id} not found for subsection_count check"));
            let section = doc.sections.iter()
                .find(|s| s.block == sec_block)
                .unwrap_or_else(|| panic!(
                    "document {document_id} has no section with block {section_block_id}"
                ));
            assert_eq!(
                section.subsections.len(), *count,
                "document {document_id} section {section_block_id} subsection count mismatch"
            );
        }

        Assertion::EventEmitted { event, fields } => {
            let CommandOutcome::Success { event_name, event_fields } = outcome else {
                panic!("event_emitted assertion requires a successful outcome");
            };
            assert_eq!(
                event_name, event,
                "expected event '{event}', got '{event_name}'"
            );
            for (key, expected_val) in fields {
                let actual_val = event_fields.get(key).unwrap_or_else(|| {
                    panic!("event '{event}' missing field '{key}' in {event_fields:?}")
                });
                assert_eq!(
                    actual_val, expected_val,
                    "event '{event}' field '{key}' mismatch"
                );
            }
        }
    }
}
