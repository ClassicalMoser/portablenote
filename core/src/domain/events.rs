use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Block events --

/// Emitted after a block is created and ready to persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAdded {
    pub block_id: Uuid,
    pub name: String,
}

/// Emitted after a block rename, including propagated block-reference link updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRenamed {
    pub block_id: Uuid,
    pub old_name: String,
    pub new_name: String,
    pub refs_updated: usize,
}

/// Emitted after a block's content body is replaced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockContentMutated {
    pub block_id: Uuid,
}

/// Emitted after a block is deleted (safe or cascade).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDeleted {
    pub block_id: Uuid,
    pub edges_removed: usize,
    pub inline_refs_reverted: usize,
}

// -- Document events --

/// Emitted after a new document definition is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAdded {
    pub document_id: Uuid,
    pub root_block_id: Uuid,
}

/// Emitted after a section or subsection is appended to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionAppended {
    pub document_id: Uuid,
    pub block_id: Uuid,
    /// 1 = top-level section, 2 = subsection.
    pub depth: u8,
}

/// Emitted after a section is removed from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRemoved {
    pub document_id: Uuid,
    pub block_id: Uuid,
}

/// Emitted after a document's sections are reordered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionsReordered {
    pub document_id: Uuid,
}

/// Emitted after a document definition is deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDeleted {
    pub document_id: Uuid,
}

// -- Edge events --

/// Emitted after a directed edge is created in the reference graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAdded {
    pub edge_id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

/// Emitted after an edge is removed from the reference graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRemoved {
    pub edge_id: Uuid,
}

// -- Vault lifecycle events --

/// Emitted when a vault is opened and its checksum has been verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultOpened {
    pub vault_id: Uuid,
    pub checksum_valid: bool,
}

/// Emitted when the on-disk checksum does not match the recomputed value,
/// indicating external modification since the last compliant save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumMismatch {
    pub expected: String,
    pub actual: String,
}

/// Envelope for all domain events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    BlockAdded(BlockAdded),
    BlockRenamed(BlockRenamed),
    BlockContentMutated(BlockContentMutated),
    BlockDeleted(BlockDeleted),
    DocumentAdded(DocumentAdded),
    SectionAppended(SectionAppended),
    SectionRemoved(SectionRemoved),
    SectionsReordered(SectionsReordered),
    DocumentDeleted(DocumentDeleted),
    EdgeAdded(EdgeAdded),
    EdgeRemoved(EdgeRemoved),
    VaultOpened(VaultOpened),
    ChecksumMismatch(ChecksumMismatch),
}
