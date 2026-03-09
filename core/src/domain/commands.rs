use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Block commands --

/// Create a new block with the given name and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlock {
    pub id: Uuid,
    pub name: String,
    pub content: String,
}

/// Change a block's human-readable name. Propagates block-reference link updates
/// across all referencing blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameBlock {
    pub block_id: Uuid,
    pub new_name: String,
}

/// Replace a block's content body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateBlockContent {
    pub block_id: Uuid,
    pub content: String,
}

/// Delete a block, rejected if incoming edges exist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlockSafe {
    pub block_id: Uuid,
}

/// Delete a block unconditionally, removing all edges (incoming + outgoing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlockCascade {
    pub block_id: Uuid,
}

// -- Document commands --

/// Create a new document rooted at the given block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocument {
    pub id: Uuid,
    pub root: Uuid,
}

/// Append a block as a top-level section (depth 2) in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendSection {
    pub document_id: Uuid,
    pub block_id: Uuid,
}

/// Append a block as a subsection (depth 3) under an existing section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendSubsection {
    pub document_id: Uuid,
    pub section_block_id: Uuid,
    pub block_id: Uuid,
}

/// Remove a top-level section from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveSection {
    pub document_id: Uuid,
    pub block_id: Uuid,
}

/// Reorder the top-level sections in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderSections {
    pub document_id: Uuid,
    pub section_order: Vec<Uuid>,
}

/// Delete a document definition. Does not affect blocks or the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDocument {
    pub document_id: Uuid,
}

// -- Edge commands --

/// Create a directed edge between two blocks in the reference graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEdge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

/// Remove an edge from the reference graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveEdge {
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
