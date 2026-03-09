use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::blocks;
use super::content;
use super::error::{Violation, ViolationDetails};
use super::types::Vault;

/// Validate all spec invariants plus structural checks.
/// Returns an empty vec if the vault is fully conformant.
///
/// Checksum integrity is deliberately excluded here — it is a
/// persistence-boundary check, meaningful only when comparing a freshly loaded
/// vault against its on-disk state. After any in-memory mutation the checksum
/// is stale by design. Use `checksum::is_drifted` at load time instead.
pub fn validate_vault(vault: &Vault) -> Vec<Violation> {
    let mut violations = Vec::new();

    check_block_metadata(vault, &mut violations);
    check_edge_endpoints(vault, &mut violations);
    check_edges_are_block_to_block(vault, &mut violations);
    check_document_block_refs(vault, &mut violations);
    check_document_acyclicity(vault, &mut violations);
    check_block_ref_edges(vault, &mut violations);
    check_name_uniqueness(vault, &mut violations);
    check_no_headings_in_content(vault, &mut violations);

    violations
}

/// Every block must have a non-empty name, no `[` or `]`, and no `%`.
fn check_block_metadata(vault: &Vault, violations: &mut Vec<Violation>) {
    for block in vault.blocks.values() {
        if block.name.is_empty() {
            violations.push(Violation {
                description: "Block metadata missing required 'name' field".to_string(),
                details: ViolationDetails::MissingMetadataField {
                    block_id: block.id,
                    missing_field: "name".to_string(),
                },
            });
        }
        if block.name.contains('[') || block.name.contains(']') {
            violations.push(Violation {
                description: "Block name contains reserved character '[' or ']'".to_string(),
                details: ViolationDetails::NameContainsReservedCharacters {
                    block_id: block.id,
                    name: block.name.clone(),
                },
            });
        }
        if blocks::name_contains_percent(&block.name) {
            violations.push(Violation {
                description: "Block name contains '%' (reserved for filename encoding)".to_string(),
                details: ViolationDetails::NameContainsPercent {
                    block_id: block.id,
                    name: block.name.clone(),
                },
            });
        }
    }
}

/// Every UUID in block-graph.json source or target fields must exist in the heap.
fn check_edge_endpoints(vault: &Vault, violations: &mut Vec<Violation>) {
    for edge in &vault.graph.edges {
        if !vault.blocks.contains_key(&edge.source) {
            violations.push(Violation {
                description: "Edge source UUID does not exist in heap".to_string(),
                details: ViolationDetails::DanglingEdgeUuid {
                    edge_id: edge.id,
                    dangling_uuid: edge.source,
                    field: "source".to_string(),
                },
            });
        }
        if !vault.blocks.contains_key(&edge.target) {
            violations.push(Violation {
                description: "Edge target UUID does not exist in heap".to_string(),
                details: ViolationDetails::DanglingEdgeUuid {
                    edge_id: edge.id,
                    dangling_uuid: edge.target,
                    field: "target".to_string(),
                },
            });
        }
    }
}

/// Edges must connect blocks to blocks — every endpoint must be a block.
/// This is a type constraint, distinct from `check_edge_endpoints` which is a
/// referential integrity check. In the current model they produce identical
/// results because only blocks exist in the heap. They diverge if non-block
/// entities (composition nodes, etc.) are later added to the UUID space.
fn check_edges_are_block_to_block(vault: &Vault, violations: &mut Vec<Violation>) {
    for edge in &vault.graph.edges {
        if !vault.blocks.contains_key(&edge.source) || !vault.blocks.contains_key(&edge.target) {
            violations.push(Violation {
                description: "Edge endpoint is not a block".to_string(),
                details: ViolationDetails::InvalidEdgeEndpoint {
                    edge_id: edge.id,
                },
            });
        }
    }
}

/// Every block UUID in a document's root, sections, or subsections must exist in the heap.
fn check_document_block_refs(vault: &Vault, violations: &mut Vec<Violation>) {
    for doc in vault.documents.values() {
        if !vault.blocks.contains_key(&doc.root) {
            violations.push(Violation {
                description: "Document root UUID does not exist in heap".to_string(),
                details: ViolationDetails::DanglingDocumentUuid {
                    document_id: doc.id,
                    dangling_uuid: doc.root,
                    field: "root".to_string(),
                },
            });
        }
        for section in &doc.sections {
            if !vault.blocks.contains_key(&section.block) {
                violations.push(Violation {
                    description: "Document section UUID does not exist in heap".to_string(),
                    details: ViolationDetails::DanglingDocumentUuid {
                        document_id: doc.id,
                        dangling_uuid: section.block,
                        field: "section".to_string(),
                    },
                });
            }
            for sub in &section.subsections {
                if !vault.blocks.contains_key(&sub.block) {
                    violations.push(Violation {
                        description: "Document subsection UUID does not exist in heap".to_string(),
                        details: ViolationDetails::DanglingDocumentUuid {
                            document_id: doc.id,
                            dangling_uuid: sub.block,
                            field: "subsection".to_string(),
                        },
                    });
                }
            }
        }
    }
}

/// No block may appear more than once in a document's hierarchy (root, sections, subsections).
fn check_document_acyclicity(vault: &Vault, violations: &mut Vec<Violation>) {
    for doc in vault.documents.values() {
        let mut seen = HashSet::new();
        seen.insert(doc.root);

        for section in &doc.sections {
            if !seen.insert(section.block) {
                violations.push(Violation {
                    description: "Block appears multiple times in document hierarchy".to_string(),
                    details: ViolationDetails::DocumentCycle {
                        document_id: doc.id,
                        block_id: section.block,
                    },
                });
            }
            for sub in &section.subsections {
                if !seen.insert(sub.block) {
                    violations.push(Violation {
                        description: "Block appears multiple times in document hierarchy"
                            .to_string(),
                        details: ViolationDetails::DocumentCycle {
                            document_id: doc.id,
                            block_id: sub.block,
                        },
                    });
                }
            }
        }
    }
}

/// Every block-reference link must have a corresponding edge and target must exist in heap.
fn check_block_ref_edges(vault: &Vault, violations: &mut Vec<Violation>) {
    let edge_set: HashSet<(Uuid, Uuid)> = vault
        .graph
        .edges
        .iter()
        .map(|e| (e.source, e.target))
        .collect();

    for (block_id, refs) in &vault.block_refs {
        for (display, target_id) in refs {
            if !vault.blocks.contains_key(target_id) {
                violations.push(Violation {
                    description: format!(
                        "Block reference [{}](block:{}) target does not exist in heap",
                        display,
                        target_id
                    ),
                    details: ViolationDetails::DanglingFooterAnnotation {
                        block_id: *block_id,
                        name: display.clone(),
                    },
                });
            } else if !edge_set.contains(&(*block_id, *target_id)) {
                violations.push(Violation {
                    description: format!(
                        "Block reference [{}](block:{}) has no corresponding edge in block-graph.json",
                        display,
                        target_id
                    ),
                    details: ViolationDetails::MissingEdgeForRef {
                        block_id: *block_id,
                        referenced_name: display.clone(),
                        target_id: *target_id,
                    },
                });
            }
        }
    }
}

/// Block names must be vault-wide unique (case-insensitive).
/// "Meeting Notes" and "meeting notes" are considered duplicates.
fn check_name_uniqueness(vault: &Vault, violations: &mut Vec<Violation>) {
    let mut names: HashMap<String, Vec<(String, Uuid)>> = HashMap::new();

    for block in vault.blocks.values() {
        if !block.name.is_empty() {
            names
                .entry(block.name.to_lowercase())
                .or_default()
                .push((block.name.clone(), block.id));
        }
    }

    for entries in names.values() {
        if entries.len() > 1 {
            let display_name = &entries[0].0;
            let ids: Vec<Uuid> = entries.iter().map(|(_, id)| *id).collect();
            violations.push(Violation {
                description: format!("Multiple blocks share the name '{display_name}' (case-insensitive)"),
                details: ViolationDetails::DuplicateName {
                    name: display_name.clone(),
                    block_ids: ids,
                },
            });
        }
    }
}

/// No block content may contain heading syntax (h1-h6) outside fenced code blocks.
fn check_no_headings_in_content(vault: &Vault, violations: &mut Vec<Violation>) {
    for block in vault.blocks.values() {
        if let Some((level, text)) = content::find_heading_outside_fence(&block.content) {
            violations.push(Violation {
                description: "Block content contains heading syntax outside fenced code block"
                    .to_string(),
                details: ViolationDetails::HeadingInContent {
                    block_id: block.id,
                    heading_text: text,
                    heading_level: level,
                },
            });
        }
    }
}
