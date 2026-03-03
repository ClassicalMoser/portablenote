use std::collections::HashMap;

use uuid::Uuid;

use super::types::{Block, Document, Section, Subsection};

pub fn create(id: Uuid, root: Uuid) -> Document {
    Document {
        id,
        root,
        sections: Vec::new(),
    }
}

/// Project a document into an ordered list of `(heading_level, block)` pairs.
///
/// Level mapping: root → 1, section → 2, subsection → 3.
/// Blocks missing from the heap are silently skipped.
pub fn project<'a>(doc: &Document, blocks: &'a HashMap<Uuid, Block>) -> Vec<(u8, &'a Block)> {
    let mut result = Vec::new();

    if let Some(root) = blocks.get(&doc.root) {
        result.push((1u8, root));
    }

    for section in &doc.sections {
        if let Some(block) = blocks.get(&section.block) {
            result.push((2u8, block));
        }
        for sub in &section.subsections {
            if let Some(block) = blocks.get(&sub.block) {
                result.push((3u8, block));
            }
        }
    }

    result
}

/// Append a top-level section to a document. Does not validate that the block
/// exists in the heap — callers must check via `BlockStore::get` before calling.
pub fn append_section(mut doc: Document, block_id: Uuid) -> Document {
    doc.sections.push(Section {
        block: block_id,
        subsections: Vec::new(),
    });
    doc
}

/// Append a subsection under the given section block. Returns `None` if the
/// section is not found.
pub fn append_subsection(
    mut doc: Document,
    section_block_id: Uuid,
    subsection_block_id: Uuid,
) -> Option<Document> {
    let section = doc
        .sections
        .iter_mut()
        .find(|s| s.block == section_block_id)?;
    section.subsections.push(Subsection {
        block: subsection_block_id,
    });
    Some(doc)
}

/// Remove a top-level section. Returns `None` if the section is not found.
pub fn remove_section(mut doc: Document, block_id: Uuid) -> Option<Document> {
    let pos = doc.sections.iter().position(|s| s.block == block_id)?;
    doc.sections.remove(pos);
    Some(doc)
}

/// Reorder top-level sections. Returns `None` if `order` does not contain
/// exactly the same set of block UUIDs as the current sections.
pub fn reorder_sections(mut doc: Document, order: Vec<Uuid>) -> Option<Document> {
    let current: std::collections::HashSet<Uuid> =
        doc.sections.iter().map(|s| s.block).collect();
    let requested: std::collections::HashSet<Uuid> = order.iter().copied().collect();

    if current != requested || order.len() != doc.sections.len() {
        return None;
    }

    let section_map: HashMap<Uuid, Section> = doc
        .sections
        .into_iter()
        .map(|s| (s.block, s))
        .collect();

    doc.sections = order
        .into_iter()
        .map(|id| section_map[&id].clone())
        .collect();

    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_block(id: Uuid, name: &str) -> Block {
        Block {
            id,
            name: name.to_string(),
            content: String::new(),
            created: Utc::now(),
            modified: Utc::now(),
        }
    }

    const ROOT: &str = "00000000-0000-4000-a000-000000000001";
    const S1: &str = "00000000-0000-4000-a000-000000000002";
    const S2: &str = "00000000-0000-4000-a000-000000000003";
    const SUB: &str = "00000000-0000-4000-a000-000000000004";
    const DOC: &str = "00000000-0000-4000-a000-0000000000d1";

    fn ids() -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
        (
            Uuid::parse_str(ROOT).unwrap(),
            Uuid::parse_str(S1).unwrap(),
            Uuid::parse_str(S2).unwrap(),
            Uuid::parse_str(SUB).unwrap(),
            Uuid::parse_str(DOC).unwrap(),
        )
    }

    #[test]
    fn create_starts_empty() {
        let (root, _, _, _, doc_id) = ids();
        let doc = create(doc_id, root);
        assert_eq!(doc.id, doc_id);
        assert_eq!(doc.root, root);
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn project_root_only() {
        let (root, _, _, _, doc_id) = ids();
        let doc = create(doc_id, root);
        let mut blocks = HashMap::new();
        blocks.insert(root, make_block(root, "Root"));
        let projection = project(&doc, &blocks);
        assert_eq!(projection.len(), 1);
        assert_eq!(projection[0].0, 1);
        assert_eq!(projection[0].1.id, root);
    }

    #[test]
    fn project_with_section_and_subsection() {
        let (root, s1, _, sub, doc_id) = ids();
        let mut doc = create(doc_id, root);
        doc.sections.push(Section {
            block: s1,
            subsections: vec![Subsection { block: sub }],
        });
        let mut blocks = HashMap::new();
        blocks.insert(root, make_block(root, "Root"));
        blocks.insert(s1, make_block(s1, "Section1"));
        blocks.insert(sub, make_block(sub, "Sub"));
        let projection = project(&doc, &blocks);
        assert_eq!(projection.len(), 3);
        assert_eq!(projection[0].0, 1);
        assert_eq!(projection[1].0, 2);
        assert_eq!(projection[2].0, 3);
    }

    #[test]
    fn project_skips_missing_blocks() {
        let (root, s1, _, _, doc_id) = ids();
        let mut doc = create(doc_id, root);
        doc.sections.push(Section {
            block: s1,
            subsections: vec![],
        });
        // Only root in heap; s1 is absent
        let mut blocks = HashMap::new();
        blocks.insert(root, make_block(root, "Root"));
        let projection = project(&doc, &blocks);
        assert_eq!(projection.len(), 1);
    }

    #[test]
    fn append_section_adds_entry() {
        let (root, s1, _, _, doc_id) = ids();
        let doc = create(doc_id, root);
        let doc = append_section(doc, s1);
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].block, s1);
    }

    #[test]
    fn append_subsection_adds_under_section() {
        let (root, s1, _, sub, doc_id) = ids();
        let doc = append_section(create(doc_id, root), s1);
        let doc = append_subsection(doc, s1, sub).unwrap();
        assert_eq!(doc.sections[0].subsections[0].block, sub);
    }

    #[test]
    fn append_subsection_missing_section_returns_none() {
        let (root, s1, _, sub, doc_id) = ids();
        let doc = create(doc_id, root);
        assert!(append_subsection(doc, s1, sub).is_none());
    }

    #[test]
    fn remove_section_removes_it() {
        let (root, s1, _, _, doc_id) = ids();
        let doc = append_section(create(doc_id, root), s1);
        let doc = remove_section(doc, s1).unwrap();
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn remove_section_missing_returns_none() {
        let (root, s1, _, _, doc_id) = ids();
        let doc = create(doc_id, root);
        assert!(remove_section(doc, s1).is_none());
    }

    #[test]
    fn reorder_sections_valid() {
        let (root, s1, s2, _, doc_id) = ids();
        let doc = append_section(append_section(create(doc_id, root), s1), s2);
        let doc = reorder_sections(doc, vec![s2, s1]).unwrap();
        assert_eq!(doc.sections[0].block, s2);
        assert_eq!(doc.sections[1].block, s1);
    }

    #[test]
    fn reorder_sections_wrong_set_returns_none() {
        let (root, s1, s2, sub, doc_id) = ids();
        let doc = append_section(append_section(create(doc_id, root), s1), s2);
        // `sub` is not in sections
        assert!(reorder_sections(doc, vec![s1, sub]).is_none());
    }
}
