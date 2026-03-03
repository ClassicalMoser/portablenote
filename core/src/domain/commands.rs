use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlock {
    pub base_version: u64,
    pub id: Uuid,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameBlock {
    pub base_version: u64,
    pub block_id: Uuid,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateBlockContent {
    pub base_version: u64,
    pub block_id: Uuid,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlockSafe {
    pub base_version: u64,
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlockCascade {
    pub base_version: u64,
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocument {
    pub base_version: u64,
    pub id: Uuid,
    pub root: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendSection {
    pub base_version: u64,
    pub document_id: Uuid,
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendSubsection {
    pub base_version: u64,
    pub document_id: Uuid,
    pub section_block_id: Uuid,
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveSection {
    pub base_version: u64,
    pub document_id: Uuid,
    pub block_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderSections {
    pub base_version: u64,
    pub document_id: Uuid,
    pub section_order: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDocument {
    pub base_version: u64,
    pub document_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEdge {
    pub base_version: u64,
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveEdge {
    pub base_version: u64,
    pub edge_id: Uuid,
}

/// Envelope for dispatching any command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    AddBlock(AddBlock),
    RenameBlock(RenameBlock),
    MutateBlockContent(MutateBlockContent),
    DeleteBlockSafe(DeleteBlockSafe),
    DeleteBlockCascade(DeleteBlockCascade),
    AddDocument(AddDocument),
    AppendSection(AppendSection),
    AppendSubsection(AppendSubsection),
    RemoveSection(RemoveSection),
    ReorderSections(ReorderSections),
    DeleteDocument(DeleteDocument),
    AddEdge(AddEdge),
    RemoveEdge(RemoveEdge),
}
