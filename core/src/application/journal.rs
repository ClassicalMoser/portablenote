//! Journal and in-memory apply for §5a commit protocol.
//!
//! Builds the journal (expected_checksum, before_image, writes) and applies
//! writes to a vault in memory for checksum computation. Recovery is
//! implemented by the adapter using the journal file and this module's types.

use uuid::Uuid;

use crate::application::results::VaultWrite;
use crate::domain::checksum;
use crate::domain::types::{Block, Document, Edge, Vault};

/// Apply `writes` to a copy of the vault; returns the new state (manifest unchanged).
/// Used to compute `expected_checksum` for the journal.
pub fn apply_writes_to_vault(vault: &Vault, writes: &[VaultWrite]) -> Vault {
    let mut v = vault.clone();
    for w in writes {
        match w {
            VaultWrite::WriteBlock(block) => {
                v.blocks.insert(block.id, block.clone());
            }
            VaultWrite::DeleteBlock(id) => {
                v.blocks.remove(id);
            }
            VaultWrite::WriteEdge(edge) => {
                v.graph.edges.retain(|e| e.id != edge.id);
                v.graph.edges.push(edge.clone());
            }
            VaultWrite::RemoveEdge(id) => {
                v.graph.edges.retain(|e| e.id != *id);
            }
            VaultWrite::WriteDocument(doc) => {
                v.documents.insert(doc.id, doc.clone());
            }
            VaultWrite::DeleteDocument(id) => {
                v.documents.remove(id);
            }
            VaultWrite::SetName { name, id } => {
                v.names.insert(name.clone(), *id);
            }
            VaultWrite::RemoveName(name) => {
                v.names.remove(name);
            }
        }
    }
    v
}

/// Compute the checksum the vault will have after `writes` are applied.
pub fn expected_checksum_after_writes(vault: &Vault, writes: &[VaultWrite]) -> String {
    let after = apply_writes_to_vault(vault, writes);
    checksum::compute(&after)
}

/// One entry in the before_image: state of an artifact before the commit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum BeforeImageEntry {
    Block(Option<Block>),
    Edge(Option<Edge>),
    Document(Option<Document>),
    /// Name entry: id is Some if the name existed, None if it was absent (undo = remove).
    Name { name: String, id: Option<Uuid> },
}

/// Journal file content per §5a. Written before apply; used for recovery.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Journal {
    pub expected_checksum: String,
    pub before_image: Vec<BeforeImageEntry>,
    pub writes: Vec<VaultWrite>,
}

/// Build the journal (expected_checksum + before_image + writes) from current vault and writes.
pub fn build_journal(vault: &Vault, writes: &[VaultWrite]) -> Journal {
    let expected_checksum = expected_checksum_after_writes(vault, writes);
    let before_image = build_before_image(vault, writes);
    let writes = writes.to_vec();
    Journal {
        expected_checksum,
        before_image,
        writes,
    }
}

fn build_before_image(vault: &Vault, writes: &[VaultWrite]) -> Vec<BeforeImageEntry> {
    let mut out = Vec::with_capacity(writes.len());
    for w in writes {
        let entry = match w {
            VaultWrite::WriteBlock(block) => BeforeImageEntry::Block(vault.blocks.get(&block.id).cloned()),
            VaultWrite::DeleteBlock(id) => BeforeImageEntry::Block(vault.blocks.get(id).cloned()),
            VaultWrite::WriteEdge(edge) => BeforeImageEntry::Edge(
                vault.graph.edges.iter().find(|e| e.id == edge.id).cloned(),
            ),
            VaultWrite::RemoveEdge(id) => BeforeImageEntry::Edge(
                vault.graph.edges.iter().find(|e| e.id == *id).cloned(),
            ),
            VaultWrite::WriteDocument(doc) => {
                BeforeImageEntry::Document(vault.documents.get(&doc.id).cloned())
            }
            VaultWrite::DeleteDocument(id) => {
                BeforeImageEntry::Document(vault.documents.get(id).cloned())
            }
            VaultWrite::SetName { name, id: _ } => BeforeImageEntry::Name {
                name: name.clone(),
                id: vault.names.get(name).copied(),
            },
            VaultWrite::RemoveName(name) => BeforeImageEntry::Name {
                name: name.clone(),
                id: vault.names.get(name).copied(),
            },
        };
        out.push(entry);
    }
    out
}

/// Recovery outcome: what the adapter must do after reading the journal and current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryCase {
    /// Writes landed, manifest lost. Rewrite manifest, delete journal.
    A,
    /// No writes landed. Delete journal.
    B,
    /// Partial writes. Restore before_image, rewrite manifest, delete journal.
    C,
}

/// Determine recovery case from current checksum, journal, and manifest.
pub fn recovery_case(
    actual_checksum: &str,
    journal: &Journal,
    manifest_checksum: &str,
) -> RecoveryCase {
    if actual_checksum == journal.expected_checksum {
        RecoveryCase::A
    } else if actual_checksum == manifest_checksum {
        RecoveryCase::B
    } else {
        RecoveryCase::C
    }
}

/// Produce the writes that restore state from before_image (Case C undo).
pub fn undo_writes_from_journal(journal: &Journal) -> Vec<VaultWrite> {
    let mut out = Vec::with_capacity(journal.before_image.len());
    for (entry, write) in journal.before_image.iter().zip(journal.writes.iter()) {
        let undo = match (entry, write) {
            (BeforeImageEntry::Block(opt), VaultWrite::WriteBlock(block)) => {
                opt.clone().map(VaultWrite::WriteBlock).or_else(|| Some(VaultWrite::DeleteBlock(block.id)))
            }
            (BeforeImageEntry::Block(opt), VaultWrite::DeleteBlock(_)) => {
                opt.clone().map(VaultWrite::WriteBlock)
            }
            (BeforeImageEntry::Edge(opt), VaultWrite::WriteEdge(edge)) => {
                opt.clone().map(VaultWrite::WriteEdge).or_else(|| Some(VaultWrite::RemoveEdge(edge.id)))
            }
            (BeforeImageEntry::Edge(opt), VaultWrite::RemoveEdge(_)) => opt.clone().map(VaultWrite::WriteEdge),
            (BeforeImageEntry::Document(opt), VaultWrite::WriteDocument(doc)) => {
                opt.clone().map(VaultWrite::WriteDocument).or_else(|| Some(VaultWrite::DeleteDocument(doc.id)))
            }
            (BeforeImageEntry::Document(opt), VaultWrite::DeleteDocument(_)) => {
                opt.clone().map(VaultWrite::WriteDocument)
            }
            (BeforeImageEntry::Name { name, id }, VaultWrite::SetName { .. }) => Some(match id {
                Some(id) => VaultWrite::SetName { name: name.clone(), id: *id },
                None => VaultWrite::RemoveName(name.clone()),
            }),
            (BeforeImageEntry::Name { name, id }, VaultWrite::RemoveName(_)) => id
                .as_ref()
                .map(|id| VaultWrite::SetName { name: name.clone(), id: *id }),
            _ => None,
        };
        if let Some(w) = undo {
            out.push(w);
        }
    }
    out
}
