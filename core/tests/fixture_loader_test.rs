mod common;

use portablenote_core::checksum;

#[test]
fn valid_vaults_have_no_drift() {
    for name in &["minimal", "with-refs", "with-documents", "with-orphans"] {
        let dir = common::spec_dir().join("valid").join(name);
        let vault = common::load_vault(&dir);
        assert!(
            !checksum::is_drifted(&vault),
            "valid vault '{name}' should not report drift"
        );
    }
}

#[test]
fn bad_checksum_vault_is_detected_as_drifted() {
    let dir = common::spec_dir().join("invalid").join("bad-checksum");
    let vault = common::load_vault(&dir);
    assert!(
        checksum::is_drifted(&vault),
        "bad-checksum vault should be detected as drifted at load time"
    );
}

#[test]
fn load_minimal_vault() {
    let dir = common::spec_dir().join("valid").join("minimal");
    let vault = common::load_vault(&dir);

    assert_eq!(vault.manifest.spec_version, "0.1.0");
    assert_eq!(vault.manifest.format, "markdown");
    assert_eq!(vault.blocks.len(), 1);
    assert_eq!(vault.graph.edges.len(), 0);
    assert_eq!(vault.documents.len(), 0);

    let block = vault.blocks.values().next().unwrap();
    assert_eq!(block.name, "Welcome");
    assert!(block.content.contains("minimal PortableNote vault"));
}

#[test]
fn load_with_refs_vault() {
    let dir = common::spec_dir().join("valid").join("with-refs");
    let vault = common::load_vault(&dir);

    assert_eq!(vault.blocks.len(), 3);
    assert_eq!(vault.graph.edges.len(), 2);
    assert_eq!(vault.documents.len(), 0);
    assert_eq!(vault.manifest.names.len(), 3);

    let core_concepts_id = vault.manifest.names.get("Core Concepts").unwrap();
    let block = &vault.blocks[core_concepts_id];
    assert!(block.content.contains("[[Getting Started]]"));
    assert!(block.content.contains("<!-- refs -->"));
}

#[test]
fn load_with_documents_vault() {
    let dir = common::spec_dir().join("valid").join("with-documents");
    let vault = common::load_vault(&dir);

    assert_eq!(vault.blocks.len(), 4);
    assert_eq!(vault.graph.edges.len(), 1);
    assert_eq!(vault.documents.len(), 1);

    let doc = vault.documents.values().next().unwrap();
    assert_eq!(doc.sections.len(), 2);
    assert_eq!(doc.sections[1].subsections.len(), 1);
}

#[test]
fn load_with_orphans_vault() {
    let dir = common::spec_dir().join("valid").join("with-orphans");
    let vault = common::load_vault(&dir);

    assert_eq!(vault.blocks.len(), 3);
    assert_eq!(vault.graph.edges.len(), 1);
}

#[test]
fn load_invalid_dangling_uuid() {
    let dir = common::spec_dir().join("invalid").join("dangling-uuid");
    let vault = common::load_vault(&dir);

    assert_eq!(vault.blocks.len(), 1);
    assert_eq!(vault.graph.edges.len(), 1);

    let expected = common::load_expected_error(&dir);
    assert_eq!(expected["invariant"], 1);
}

#[test]
fn load_invalid_missing_metadata() {
    let dir = common::spec_dir().join("invalid").join("missing-frontmatter");
    let vault = common::load_vault(&dir);

    assert_eq!(vault.blocks.len(), 1);
    let block = vault.blocks.values().next().unwrap();
    assert!(block.name.is_empty(), "Missing name should result in empty string");
}
