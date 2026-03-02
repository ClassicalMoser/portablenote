use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAdded {
    pub block_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRenamed {
    pub block_id: Uuid,
    pub old_name: String,
    pub new_name: String,
    pub refs_updated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockContentMutated {
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDeleted {
    pub block_id: Uuid,
    pub edges_removed: usize,
    pub inline_refs_reverted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAdded {
    pub document_id: Uuid,
    pub root_block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionAppended {
    pub document_id: Uuid,
    pub block_id: Uuid,
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRemoved {
    pub document_id: Uuid,
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionsReordered {
    pub document_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDeleted {
    pub document_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAdded {
    pub edge_id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRemoved {
    pub edge_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultOpened {
    pub vault_id: Uuid,
    pub checksum_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumMismatch {
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConflict {
    pub command_type: String,
    pub base_version: u64,
    pub current_version: u64,
    pub artifact_id: Uuid,
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
    SaveConflict(SaveConflict),
}
