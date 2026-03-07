//! Integration tests for §5a: manifest update and journal recovery.

use std::process::Command;

use portablenote_core::application::journal::Journal;
use portablenote_core::domain::checksum;
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
