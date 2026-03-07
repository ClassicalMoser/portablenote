use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use super::types::Vault;

fn normalize_str(s: &str) -> String {
    s.replace("\r\n", "\n").nfc().collect()
}

/// Compute the canonical SHA256 checksum of a vault's source artifacts.
///
/// Canonical serialization order:
///   1. Blocks: sorted by UUID (lexicographic on the hyphenated string).
///      Each block contributes: `block:<uuid>\n<name>\n<content>\n`
///   2. Edges: sorted by UUID.
///      Each edge contributes: `edge:<uuid>\n<source>-><target>\n`
///   3. Documents: sorted by UUID.
///      Each document contributes: `doc:<uuid>\nroot:<root_uuid>\n`
///      followed by sections in their declared order (order is semantically
///      significant for compositions — not sorted):
///      `section:<block_uuid>\n`, then for each subsection `sub:<block_uuid>\n`
///
/// All string fields are NFC-normalized and line endings are normalized to LF
/// before hashing, per spec §1 Normalization Rules.
///
/// The result is `sha256:<hex>`.
///
/// Note: `names.json` is deliberately excluded — it is derived from block
/// metadata and can be reconstructed by scanning `/blocks`.
pub fn compute(vault: &Vault) -> String {
    let mut hasher = Sha256::new();

    let mut block_ids: Vec<&Uuid> = vault.blocks.keys().collect();
    block_ids.sort();

    for id in block_ids {
        let block = &vault.blocks[id];
        let name = normalize_str(&block.name);
        let content = normalize_str(&block.content);
        hasher.update(format!("block:{}\n{}\n{}\n", id, name, content).as_bytes());
    }

    let mut edges = vault.graph.edges.clone();
    edges.sort_by(|a, b| a.id.cmp(&b.id));

    for edge in &edges {
        hasher.update(format!("edge:{}\n{}->{}\n", edge.id, edge.source, edge.target).as_bytes());
    }

    let mut doc_ids: Vec<&Uuid> = vault.documents.keys().collect();
    doc_ids.sort();

    for id in doc_ids {
        let doc = &vault.documents[id];
        hasher.update(format!("doc:{}\nroot:{}\n", id, doc.root).as_bytes());
        for section in &doc.sections {
            hasher.update(format!("section:{}\n", section.block).as_bytes());
            for sub in &section.subsections {
                hasher.update(format!("sub:{}\n", sub.block).as_bytes());
            }
        }
    }

    let hash = hasher.finalize();
    format!("sha256:{hash:x}")
}

/// Returns `true` if the vault's source artifacts no longer match the checksum
/// stored in `manifest.checksum`.
///
/// This is a **load-time / persistence-boundary check** for detecting manual
/// file drift (edits made outside the application). It is not an in-memory
/// mutation invariant — the checksum is stale by definition between mutation
/// and the next save. Call this after loading a vault from disk, not during
/// domain validation.
pub fn is_drifted(vault: &Vault) -> bool {
    compute(vault) != vault.manifest.checksum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_block(id: Uuid, name: &str, content: &str) -> Block {
        Block {
            id,
            name: name.to_string(),
            content: content.to_string(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    fn empty_vault() -> Vault {
        Vault {
            manifest: Manifest {
                vault_id: Uuid::nil(),
                spec_version: "0.1.0".to_string(),
                format: "markdown".to_string(),
                checksum: String::new(),
                previous_checksum: None,
            },
            blocks: HashMap::new(),
            graph: BlockGraph {
                version: "0.1.0".to_string(),
                edges: Vec::new(),
            },
            documents: HashMap::new(),
            names: HashMap::new(),
            version: 0,
        }
    }

    #[test]
    fn starts_with_sha256_prefix() {
        let vault = empty_vault();
        let result = compute(&vault);
        assert!(result.starts_with("sha256:"), "got: {result}");
    }

    #[test]
    fn empty_vault_is_deterministic() {
        let a = compute(&empty_vault());
        let b = compute(&empty_vault());
        assert_eq!(a, b);
    }

    #[test]
    fn different_content_different_checksum() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();

        let mut v1 = empty_vault();
        v1.blocks.insert(id, make_block(id, "A", "hello"));

        let mut v2 = empty_vault();
        v2.blocks.insert(id, make_block(id, "A", "world"));

        assert_ne!(compute(&v1), compute(&v2));
    }

    #[test]
    fn order_independent_of_insertion() {
        let id_a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let id_b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();

        let mut v1 = empty_vault();
        v1.blocks.insert(id_a, make_block(id_a, "A", "aaa"));
        v1.blocks.insert(id_b, make_block(id_b, "B", "bbb"));

        let mut v2 = empty_vault();
        v2.blocks.insert(id_b, make_block(id_b, "B", "bbb"));
        v2.blocks.insert(id_a, make_block(id_a, "A", "aaa"));

        assert_eq!(compute(&v1), compute(&v2));
    }

    #[test]
    fn edges_affect_checksum() {
        let id_a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let id_b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let edge_id = Uuid::parse_str("00000000-0000-4000-a000-0000000000e1").unwrap();

        let mut v1 = empty_vault();
        v1.blocks.insert(id_a, make_block(id_a, "A", ""));
        v1.blocks.insert(id_b, make_block(id_b, "B", ""));

        let mut v2 = v1.clone();
        v2.graph.edges.push(Edge {
            id: edge_id,
            source: id_a,
            target: id_b,
        });

        assert_ne!(compute(&v1), compute(&v2));
    }

    #[test]
    fn documents_affect_checksum() {
        let id_a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let id_b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let doc_id = Uuid::parse_str("00000000-0000-4000-a000-0000000000d1").unwrap();

        let mut v1 = empty_vault();
        v1.blocks.insert(id_a, make_block(id_a, "A", ""));
        v1.blocks.insert(id_b, make_block(id_b, "B", ""));

        let mut v2 = v1.clone();
        v2.documents.insert(
            doc_id,
            Document {
                id: doc_id,
                root: id_a,
                sections: vec![Section {
                    block: id_b,
                    subsections: vec![],
                }],
            },
        );

        assert_ne!(compute(&v1), compute(&v2));
    }

    #[test]
    fn document_section_order_affects_checksum() {
        let id_a = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let id_b = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let id_c = Uuid::parse_str("00000000-0000-4000-a000-000000000003").unwrap();
        let doc_id = Uuid::parse_str("00000000-0000-4000-a000-0000000000d1").unwrap();

        let mut v1 = empty_vault();
        v1.documents.insert(
            doc_id,
            Document {
                id: doc_id,
                root: id_a,
                sections: vec![
                    Section {
                        block: id_b,
                        subsections: vec![],
                    },
                    Section {
                        block: id_c,
                        subsections: vec![],
                    },
                ],
            },
        );

        let mut v2 = empty_vault();
        v2.documents.insert(
            doc_id,
            Document {
                id: doc_id,
                root: id_a,
                sections: vec![
                    Section {
                        block: id_c,
                        subsections: vec![],
                    },
                    Section {
                        block: id_b,
                        subsections: vec![],
                    },
                ],
            },
        );

        assert_ne!(compute(&v1), compute(&v2));
    }

    #[test]
    fn crlf_and_lf_content_produce_same_checksum() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();

        let mut v_lf = empty_vault();
        v_lf.blocks
            .insert(id, make_block(id, "A", "line1\nline2\n"));

        let mut v_crlf = empty_vault();
        v_crlf
            .blocks
            .insert(id, make_block(id, "A", "line1\r\nline2\r\n"));

        assert_eq!(
            compute(&v_lf),
            compute(&v_crlf),
            "CRLF content must produce the same checksum as LF content"
        );
    }

    #[test]
    fn nfc_and_nfd_name_produce_same_checksum() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();

        // NFC: é = U+00E9 (single code point)
        let nfc_name = "caf\u{00E9}";
        // NFD: e + combining acute = U+0065 U+0301
        let nfd_name = "cafe\u{0301}";

        let mut v_nfc = empty_vault();
        v_nfc
            .blocks
            .insert(id, make_block(id, nfc_name, "content"));

        let mut v_nfd = empty_vault();
        v_nfd
            .blocks
            .insert(id, make_block(id, nfd_name, "content"));

        assert_eq!(
            compute(&v_nfc),
            compute(&v_nfd),
            "NFD name must produce the same checksum as NFC name"
        );
    }

    #[test]
    fn nfc_and_nfd_content_produce_same_checksum() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();

        let nfc_content = "caf\u{00E9} latt\u{00E9}";
        let nfd_content = "cafe\u{0301} latte\u{0301}";

        let mut v_nfc = empty_vault();
        v_nfc
            .blocks
            .insert(id, make_block(id, "A", nfc_content));

        let mut v_nfd = empty_vault();
        v_nfd
            .blocks
            .insert(id, make_block(id, "A", nfd_content));

        assert_eq!(
            compute(&v_nfc),
            compute(&v_nfd),
            "NFD content must produce the same checksum as NFC content"
        );
    }

    #[test]
    fn timestamps_do_not_affect_checksum() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();

        let mut v1 = empty_vault();
        v1.blocks.insert(id, make_block(id, "A", "content"));

        let mut v2 = empty_vault();
        let mut block = make_block(id, "A", "content");
        block.created = chrono::DateTime::from_timestamp(0, 0).unwrap();
        block.modified = chrono::DateTime::from_timestamp(0, 0).unwrap();
        v2.blocks.insert(id, block);

        assert_eq!(compute(&v1), compute(&v2));
    }
}
