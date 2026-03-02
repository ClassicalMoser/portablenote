use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::error::{Violation, ViolationDetails};
use crate::types::Vault;

/// Validate all spec invariants (section 6, items 1-8) plus structural checks.
/// Returns an empty vec if the vault is fully conformant.
///
/// Invariant 9 (checksum integrity) is deliberately excluded here. It is a
/// persistence-boundary check — meaningful only when comparing a freshly loaded
/// vault against its on-disk state. After any in-memory mutation the checksum
/// is stale by design. Use `checksum::is_drifted` at load time instead.
pub fn validate_vault(vault: &Vault) -> Vec<Violation> {
    let mut violations = Vec::new();

    check_structural(vault, &mut violations);
    check_invariant_1(vault, &mut violations);
    check_invariant_2(vault, &mut violations);
    check_invariant_3(vault, &mut violations);
    check_invariant_4(vault, &mut violations);
    check_invariant_5(vault, &mut violations);
    check_invariant_6(vault, &mut violations);
    check_invariant_7(vault, &mut violations);
    check_invariant_8(vault, &mut violations);

    violations
}

/// Structural: every block must have a non-empty name.
fn check_structural(vault: &Vault, violations: &mut Vec<Violation>) {
    for block in vault.blocks.values() {
        if block.name.is_empty() {
            violations.push(Violation {
                invariant: None,
                description: "Block metadata missing required 'name' field".to_string(),
                details: ViolationDetails::MissingMetadataField {
                    block_id: block.id,
                    missing_field: "name".to_string(),
                },
            });
        }
    }
}

/// Invariant 1: Every UUID in block-graph.json source or target fields exists in the heap.
fn check_invariant_1(vault: &Vault, violations: &mut Vec<Violation>) {
    for edge in &vault.graph.edges {
        if !vault.blocks.contains_key(&edge.source) {
            violations.push(Violation {
                invariant: Some(1),
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
                invariant: Some(1),
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

/// Invariant 2: Every block UUID in a document's root, sections, or subsections exists in the heap.
fn check_invariant_2(vault: &Vault, violations: &mut Vec<Violation>) {
    for doc in vault.documents.values() {
        if !vault.blocks.contains_key(&doc.root) {
            violations.push(Violation {
                invariant: Some(2),
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
                    invariant: Some(2),
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
                        invariant: Some(2),
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

/// Invariant 3: No block is its own ancestor within a document (acyclicity).
/// With only two levels (section/subsection), this means a block cannot appear as
/// both a section and a subsection of itself, and root cannot also be a section.
fn check_invariant_3(vault: &Vault, violations: &mut Vec<Violation>) {
    for doc in vault.documents.values() {
        let mut seen = HashSet::new();
        seen.insert(doc.root);

        for section in &doc.sections {
            if !seen.insert(section.block) {
                violations.push(Violation {
                    invariant: Some(3),
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
                        invariant: Some(3),
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

/// Invariant 4: Every [[Name]] inline reference has a corresponding footer annotation
/// and a corresponding edge in block-graph.json.
fn check_invariant_4(vault: &Vault, violations: &mut Vec<Violation>) {
    let edge_set: HashSet<(Uuid, Uuid)> = vault
        .graph
        .edges
        .iter()
        .map(|e| (e.source, e.target))
        .collect();

    for block in vault.blocks.values() {
        let inline_refs = extract_inline_refs(&block.content);
        let footer_map = extract_footer_annotations(&block.content);

        for ref_name in &inline_refs {
            if !footer_map.contains_key(ref_name.as_str()) {
                violations.push(Violation {
                    invariant: Some(4),
                    description: format!(
                        "Inline reference [[{ref_name}]] has no footer annotation"
                    ),
                    details: ViolationDetails::MissingFooterAnnotation {
                        block_id: block.id,
                        referenced_name: ref_name.clone(),
                    },
                });
                continue;
            }

            if let Some(target_id) = footer_map.get(ref_name.as_str()) {
                if !edge_set.contains(&(block.id, *target_id)) {
                    violations.push(Violation {
                        invariant: Some(4),
                        description: format!(
                            "Inline reference [[{ref_name}]] has no corresponding edge in block-graph.json"
                        ),
                        details: ViolationDetails::MissingEdgeForRef {
                            block_id: block.id,
                            referenced_name: ref_name.clone(),
                            target_id: *target_id,
                        },
                    });
                }
            }
        }
    }
}

/// Invariant 5: Every footer annotation maps to a name that resolves to an existing block.
fn check_invariant_5(vault: &Vault, violations: &mut Vec<Violation>) {
    for block in vault.blocks.values() {
        let footer_map = extract_footer_annotations(&block.content);

        for (name, target_id) in &footer_map {
            let target_exists = vault.blocks.contains_key(target_id);
            let name_resolves = vault.manifest.names.get(*name) == Some(target_id);

            if !target_exists || !name_resolves {
                violations.push(Violation {
                    invariant: Some(5),
                    description: format!(
                        "Footer annotation [{name}] does not resolve to an existing block"
                    ),
                    details: ViolationDetails::DanglingFooterAnnotation {
                        block_id: block.id,
                        name: name.to_string(),
                    },
                });
            }
        }
    }
}

/// Invariant 6: Block names are vault-wide unique.
fn check_invariant_6(vault: &Vault, violations: &mut Vec<Violation>) {
    let mut names: HashMap<&str, Vec<Uuid>> = HashMap::new();

    for block in vault.blocks.values() {
        if !block.name.is_empty() {
            names.entry(&block.name).or_default().push(block.id);
        }
    }

    for (name, ids) in &names {
        if ids.len() > 1 {
            violations.push(Violation {
                invariant: Some(6),
                description: format!("Multiple blocks share the name '{name}'"),
                details: ViolationDetails::DuplicateName {
                    name: name.to_string(),
                    block_ids: ids.clone(),
                },
            });
        }
    }
}

/// Invariant 7: Every block file UUID matches its metadata id field.
/// In our model, the fixture loader already uses the filename UUID as the HashMap key
/// and the metadata id as `block.id`. We check they match.
fn check_invariant_7(vault: &Vault, violations: &mut Vec<Violation>) {
    for (&file_uuid, block) in &vault.blocks {
        if file_uuid != block.id {
            violations.push(Violation {
                invariant: Some(7),
                description: "Block file UUID does not match metadata id".to_string(),
                details: ViolationDetails::UuidMismatch {
                    file_uuid,
                    metadata_uuid: block.id,
                },
            });
        }
    }
}

/// Invariant 8: No block content contains heading syntax (h1-h6) outside fenced code blocks.
fn check_invariant_8(vault: &Vault, violations: &mut Vec<Violation>) {
    for block in vault.blocks.values() {
        if let Some((level, text)) = find_heading_outside_fence(&block.content) {
            violations.push(Violation {
                invariant: Some(8),
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

// --- Content parsing helpers ---

/// Extract all `[[Name]]` inline references from block content.
fn extract_inline_refs(content: &str) -> Vec<String> {
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

/// Extract footer annotations: `[Name]: uuid:<uuid>` lines after `<!-- refs -->`.
fn extract_footer_annotations(content: &str) -> HashMap<&str, Uuid> {
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
                map.insert(name, uuid);
            }
        }
    }

    map
}

/// Find the first heading (# through ######) outside a fenced code block.
/// Returns `(level, heading_text)` if found.
fn find_heading_outside_fence(content: &str) -> Option<(u8, String)> {
    let mut in_fence = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }

        if in_fence {
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix('#') {
            let mut level: u8 = 1;
            let mut rest = stripped;

            while let Some(s) = rest.strip_prefix('#') {
                level += 1;
                rest = s;
                if level > 6 {
                    break;
                }
            }

            if level <= 6 {
                if let Some(s) = rest.strip_prefix(' ') {
                    let text = s.trim().to_string();
                    if !text.is_empty() {
                        return Some((level, text));
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_inline_refs_basic() {
        let content = "See [[Getting Started]] and [[Advanced Topics]].";
        let refs = extract_inline_refs(content);
        assert_eq!(refs, vec!["Getting Started", "Advanced Topics"]);
    }

    #[test]
    fn extract_inline_refs_empty() {
        let refs = extract_inline_refs("No refs here.");
        assert!(refs.is_empty());
    }

    #[test]
    fn extract_footer_annotations_basic() {
        let content = "Some text.\n\n<!-- refs -->\n[Getting Started]: uuid:20000000-0000-4000-a000-000000000002\n[Advanced Topics]: uuid:20000000-0000-4000-a000-000000000003\n";
        let map = extract_footer_annotations(content);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("Getting Started"));
        assert!(map.contains_key("Advanced Topics"));
    }

    #[test]
    fn find_heading_outside_fence_detects() {
        let content = "Some text.\n\n## Bad Heading\n\nMore text.";
        let result = find_heading_outside_fence(content);
        assert_eq!(result, Some((2, "Bad Heading".to_string())));
    }

    #[test]
    fn find_heading_inside_fence_ignored() {
        let content = "Text.\n\n```\n## Not a heading\n```\n\nMore text.";
        let result = find_heading_outside_fence(content);
        assert!(result.is_none());
    }

    #[test]
    fn find_heading_none() {
        let content = "Plain text with no headings.";
        let result = find_heading_outside_fence(content);
        assert!(result.is_none());
    }
}
