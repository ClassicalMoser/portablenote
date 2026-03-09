//! Integration tests for §5a: manifest update and journal recovery.

use std::process::Command;

use portablenote_core::application::journal::{BeforeImageEntry, Journal};
use portablenote_core::application::ports::MutationGate;
use portablenote_core::application::results::VaultWrite;
use portablenote_core::domain::checksum;
use portablenote_core::domain::error::DomainError;
use portablenote_core::domain::types::Block;
use portablenote_infra::fs::{
    FsBlockStore, FsDocumentStore, FsGraphStore, FsJournalStore, FsManifestStore, FsMutationGate,
    FsNameIndex,
};

fn pn() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pn"))
}

#[test]
fn manifest_checksum_updated_after_add_block() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_path = dir.path();

    // init vault
    let out = pn().arg("init").arg(vault_path).output().unwrap();
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));

    // mutate: add block
    let out = pn()
        .arg("--vault")
        .arg(vault_path)
        .args(["add", "TestBlock", "--content", "some content"])
        .output()
        .unwrap();
    assert!(out.status.success(), "add failed: {}", String::from_utf8_lossy(&out.stderr));

    // load vault from disk and assert manifest reflects current state
    let pn_dir = vault_path.join("portablenote");
    let blocks = FsBlockStore::open(pn_dir.join("blocks")).unwrap();
    let graph = FsGraphStore::open(pn_dir.join("block-graph.json")).unwrap();
    let documents = FsDocumentStore::open(pn_dir.join("documents")).unwrap();
    let names = FsNameIndex::open(pn_dir.join("names.json")).unwrap();
    let manifest = FsManifestStore::open(pn_dir.join("manifest.json"));

    let gate = FsMutationGate {
        blocks: &blocks,
        graph: &graph,
        documents: &documents,
        names: &names,
        manifest: &manifest,
    };
    let vault = gate.build_vault().expect("vault should have manifest");

    let computed = checksum::compute(&vault);
    assert_eq!(
        vault.manifest.checksum, computed,
        "manifest.checksum must equal computed checksum after mutation (commit §5a)"
    );
    assert!(
        vault.manifest.previous_checksum.is_some(),
        "manifest.previous_checksum must be set after first mutation"
    );
}

/// After two adds, manifest must form a chain: previous_checksum == checksum from after first add.
#[test]
fn manifest_checksum_chain_after_two_adds() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_path = dir.path();

    let out = pn().arg("init").arg(vault_path).output().unwrap();
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));

    // First add
    let out = pn()
        .arg("--vault")
        .arg(vault_path)
        .args(["add", "First", "--content", "one"])
        .output()
        .unwrap();
    assert!(out.status.success(), "first add failed: {}", String::from_utf8_lossy(&out.stderr));

    let vault_after_first = load_vault(vault_path);
    let checksum_after_first = vault_after_first.manifest.checksum.clone();
    assert!(
        vault_after_first.manifest.previous_checksum.is_some(),
        "previous_checksum must be set after first add"
    );

    // Second add
    let out = pn()
        .arg("--vault")
        .arg(vault_path)
        .args(["add", "Second", "--content", "two"])
        .output()
        .unwrap();
    assert!(out.status.success(), "second add failed: {}", String::from_utf8_lossy(&out.stderr));

    let vault_after_second = load_vault(vault_path);
    assert_eq!(
        vault_after_second.manifest.previous_checksum.as_deref(),
        Some(checksum_after_first.as_str()),
        "previous_checksum after second add must equal checksum after first add (chain)"
    );
}

/// Recovery Case B: journal present but no writes landed (actual == manifest.checksum).
/// Open should delete the journal and leave the vault usable.
#[test]
fn recovery_deletes_journal_when_no_writes_landed() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_path = dir.path();

    let out = pn().arg("init").arg(vault_path).output().unwrap();
    assert!(out.status.success(), "init failed: {}", String::from_utf8_lossy(&out.stderr));

    let out = pn()
        .arg("--vault")
        .arg(vault_path)
        .args(["add", "X", "--content", "y"])
        .output()
        .unwrap();
    assert!(out.status.success(), "add failed: {}", String::from_utf8_lossy(&out.stderr));

    let pn_dir = vault_path.join("portablenote");
    let journal = FsJournalStore::open(&pn_dir);
    // Simulate leftover journal (e.g. crash before manifest write; we use a fake journal
    // that triggers Case B: actual == manifest.checksum != expected_checksum)
    let fake = Journal {
        expected_checksum: "sha256:fake".to_string(),
        before_image: vec![],
        writes: vec![],
    };
    journal.write(&fake).unwrap();
    assert!(journal.exists(), "journal should exist before open");

    // Open vault (list triggers load + recovery)
    let out = pn().arg("--vault").arg(vault_path).arg("list").output().unwrap();
    assert!(out.status.success(), "list failed: {}", String::from_utf8_lossy(&out.stderr));

    // Case B: actual (current checksum) == manifest.checksum, so recovery deletes journal
    assert!(!journal.exists(), "journal should be deleted after recovery");
}

fn load_vault(vault_path: &std::path::Path) -> portablenote_core::domain::types::Vault {
    let pn_dir = vault_path.join("portablenote");
    let blocks = FsBlockStore::open(pn_dir.join("blocks")).unwrap();
    let graph = FsGraphStore::open(pn_dir.join("block-graph.json")).unwrap();
    let documents = FsDocumentStore::open(pn_dir.join("documents")).unwrap();
    let names = FsNameIndex::open(pn_dir.join("names.json")).unwrap();
    let manifest = FsManifestStore::open(pn_dir.join("manifest.json"));
    let gate = FsMutationGate {
        blocks: &blocks,
        graph: &graph,
        documents: &documents,
        names: &names,
        manifest: &manifest,
    };
    gate.build_vault().expect("vault should have manifest")
}

/// Recovery Case A: writes landed but manifest was not updated.
/// The journal has the correct expected_checksum matching current state.
/// Recovery should rewrite manifest and delete journal.
#[test]
fn recovery_case_a_rewrites_manifest_when_writes_landed() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_path = dir.path();

    // init + add a block via CLI to create a real vault
    let out = pn().arg("init").arg(vault_path).output().unwrap();
    assert!(out.status.success());

    let out = pn()
        .arg("--vault").arg(vault_path)
        .args(["add", "Alpha", "--content", "hello"])
        .output().unwrap();
    assert!(out.status.success());

    let pn_dir = vault_path.join("portablenote");

    // Record pre-mutation manifest checksum
    let vault_before = load_vault(vault_path);
    let pre_checksum = vault_before.manifest.checksum.clone();

    // Manually add a second block directly to disk (simulate writes that landed)
    let new_id = uuid::Uuid::parse_str("99999999-0000-4000-a000-000000000099").unwrap();
    let now = chrono::Utc::now();
    let new_block = Block {
        id: new_id,
        name: "Beta".to_string(),
        content: "world".to_string(),
        created: now,
        modified: now,
    };
    let block_content = portablenote_core::domain::format::serialize_block_file(&new_block);
    std::fs::write(pn_dir.join("blocks/Beta.md"), &block_content).unwrap();

    // Update names.json to include new block
    let names_path = pn_dir.join("names.json");
    let mut names_map: std::collections::HashMap<String, uuid::Uuid> =
        serde_json::from_str(&std::fs::read_to_string(&names_path).unwrap()).unwrap();
    names_map.insert("Beta".to_string(), new_id);
    std::fs::write(&names_path, serde_json::to_string_pretty(&names_map).unwrap()).unwrap();

    // Compute what the checksum SHOULD be after these writes
    let vault_after = load_vault(vault_path);
    let expected_checksum = checksum::compute(&vault_after);

    // Manifest still has the OLD checksum (we didn't update it)
    assert_ne!(vault_after.manifest.checksum, expected_checksum);

    // Create a journal that matches this Case A scenario
    let journal_store = FsJournalStore::open(&pn_dir);
    let journal = Journal {
        expected_checksum: expected_checksum.clone(),
        before_image: vec![
            BeforeImageEntry::Block { data: None },
            BeforeImageEntry::Name { name: "Beta".to_string(), id: None },
        ],
        writes: vec![
            VaultWrite::WriteBlock(new_block),
            VaultWrite::SetName { name: "Beta".to_string(), id: new_id },
        ],
    };
    journal_store.write(&journal).unwrap();

    // Open vault (triggers recovery)
    let out = pn().arg("--vault").arg(vault_path).arg("list").output().unwrap();
    assert!(out.status.success(), "list failed: {}", String::from_utf8_lossy(&out.stderr));

    // Journal should be deleted
    assert!(!journal_store.exists(), "journal should be deleted after Case A recovery");

    // Manifest should now reflect the new state
    let vault_recovered = load_vault(vault_path);
    assert_eq!(
        vault_recovered.manifest.checksum, expected_checksum,
        "manifest checksum should match expected after Case A recovery"
    );
    assert_eq!(
        vault_recovered.manifest.previous_checksum.as_deref(),
        Some(pre_checksum.as_str()),
        "previous_checksum should be the old manifest checksum"
    );
}

/// Recovery Case C: partial writes — only first of two blocks landed on disk.
/// Actual checksum matches neither expected (both blocks) nor manifest (no new blocks).
/// Recovery should undo the partial write and restore vault to pre-commit state.
#[test]
fn recovery_case_c_undoes_partial_writes() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_path = dir.path();

    let out = pn().arg("init").arg(vault_path).output().unwrap();
    assert!(out.status.success());

    let out = pn()
        .arg("--vault").arg(vault_path)
        .args(["add", "Alpha", "--content", "hello"])
        .output().unwrap();
    assert!(out.status.success());

    let pn_dir = vault_path.join("portablenote");
    let vault_before = load_vault(vault_path);
    let manifest_checksum = vault_before.manifest.checksum.clone();

    // The commit was supposed to add TWO blocks (Gamma + Delta).
    // Simulate: only Gamma's block file landed. Delta did not.
    let now = chrono::Utc::now();
    let gamma_id = uuid::Uuid::parse_str("99999999-0000-4000-a000-000000000099").unwrap();
    let gamma = Block {
        id: gamma_id,
        name: "Gamma".to_string(),
        content: "partial".to_string(),
        created: now,
        modified: now,
    };
    let delta_id = uuid::Uuid::parse_str("99999999-0000-4000-a000-0000000000aa").unwrap();
    let delta = Block {
        id: delta_id,
        name: "Delta".to_string(),
        content: "also partial".to_string(),
        created: now,
        modified: now,
    };

    // Write only Gamma to disk (Delta's write "didn't land")
    let gamma_content = portablenote_core::domain::format::serialize_block_file(&gamma);
    std::fs::write(pn_dir.join("blocks/Gamma.md"), &gamma_content).unwrap();

    // Compute checksums for validation
    let vault_partial = load_vault(vault_path);
    let actual = checksum::compute(&vault_partial);
    assert_ne!(actual, manifest_checksum, "partial write changed the vault");

    // Expected checksum includes BOTH blocks
    let mut vault_full = vault_partial.clone();
    vault_full.blocks.insert(delta_id, delta.clone());
    let expected_full = checksum::compute(&vault_full);
    assert_ne!(actual, expected_full, "only Gamma landed, not Delta");

    // Create journal referencing both writes
    let journal_store = FsJournalStore::open(&pn_dir);
    let journal = Journal {
        expected_checksum: expected_full,
        before_image: vec![
            BeforeImageEntry::Block { data: None },
            BeforeImageEntry::Block { data: None },
        ],
        writes: vec![
            VaultWrite::WriteBlock(gamma.clone()),
            VaultWrite::WriteBlock(delta),
        ],
    };
    journal_store.write(&journal).unwrap();

    // Open vault (triggers recovery -> Case C -> undo)
    let out = pn().arg("--vault").arg(vault_path).arg("list").output().unwrap();
    assert!(out.status.success(), "list failed: {}", String::from_utf8_lossy(&out.stderr));

    // Journal should be deleted
    assert!(!journal_store.exists(), "journal should be deleted after Case C recovery");

    // Block Gamma should be removed by undo (before_image was None -> delete)
    let vault_recovered = load_vault(vault_path);
    assert!(
        !vault_recovered.blocks.contains_key(&gamma_id),
        "block Gamma should be removed by Case C undo"
    );
    assert!(
        !vault_recovered.blocks.contains_key(&delta_id),
        "block Delta was never written and should not exist"
    );

    // Vault should be restored to pre-commit state
    let recovered_checksum = checksum::compute(&vault_recovered);
    assert_eq!(
        recovered_checksum, manifest_checksum,
        "vault should be restored to pre-commit state after Case C undo"
    );

    // Case C recovery should have rewritten the manifest (previous_checksum updated)
    assert_eq!(
        vault_recovered.manifest.checksum, manifest_checksum,
        "manifest.checksum should match restored state"
    );
    assert!(
        vault_recovered.manifest.previous_checksum.is_some(),
        "manifest should be rewritten after Case C undo"
    );
}

/// OCC gate: when expected_checksum is set and does not match current manifest.checksum,
/// the gate returns StaleState. CLI currently calls allow_mutation(None); this test
/// covers the StaleState path end-to-end through the FS adapter.
#[test]
fn mutation_gate_returns_stale_state_when_expected_checksum_mismatches() {
    let dir = tempfile::TempDir::new().unwrap();
    let vault_path = dir.path();

    let out = pn().arg("init").arg(vault_path).output().unwrap();
    assert!(out.status.success());
    let out = pn()
        .arg("--vault")
        .arg(vault_path)
        .args(["add", "X", "--content", "y"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let vault = load_vault(vault_path);
    let actual_checksum = vault.manifest.checksum.clone();
    assert!(!actual_checksum.is_empty());

    let pn_dir = vault_path.join("portablenote");
    let blocks = FsBlockStore::open(pn_dir.join("blocks")).unwrap();
    let graph = FsGraphStore::open(pn_dir.join("block-graph.json")).unwrap();
    let documents = FsDocumentStore::open(pn_dir.join("documents")).unwrap();
    let names = FsNameIndex::open(pn_dir.join("names.json")).unwrap();
    let manifest = FsManifestStore::open(pn_dir.join("manifest.json"));

    let gate = FsMutationGate {
        blocks: &blocks,
        graph: &graph,
        documents: &documents,
        names: &names,
        manifest: &manifest,
    };

    let result = gate.allow_mutation(Some("sha256:stale_expected".to_string()));
    let err = result.unwrap_err();
    match &err {
        DomainError::StaleState { expected, actual } => {
            assert_eq!(expected.as_str(), "sha256:stale_expected");
            assert_eq!(actual.as_str(), actual_checksum.as_str());
        }
        _ => panic!("expected StaleState, got {:?}", err),
    }
}
