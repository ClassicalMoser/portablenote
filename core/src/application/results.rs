use uuid::Uuid;

use crate::domain::events::{BlockAdded, BlockDeleted, BlockRenamed};
use crate::domain::types::Block;

/// Result of adding a block. Adapter must persist the block and register the name.
pub struct AddBlockResult {
    pub block: Block,
    pub event: BlockAdded,
}

/// Result of renaming a block. Adapter must persist the renamed block,
/// all propagated blocks (with updated inline refs), remove the old name,
/// and register the new name.
pub struct RenameBlockResult {
    pub renamed: Block,
    pub propagated: Vec<Block>,
    pub old_name: String,
    pub event: BlockRenamed,
}

/// Result of a safe block deletion (rejected if incoming edges exist).
/// Adapter must persist reverted blocks, remove outgoing edges, delete the
/// block, and remove the name.
pub struct DeleteBlockSafeResult {
    pub block_id: Uuid,
    pub reverted_blocks: Vec<Block>,
    pub outgoing_edge_ids: Vec<Uuid>,
    pub name_to_remove: String,
    pub event: BlockDeleted,
}

/// Result of a cascade block deletion (always proceeds).
/// Adapter must persist reverted blocks, remove all edges (incoming + outgoing),
/// delete the block, and remove the name.
pub struct DeleteBlockCascadeResult {
    pub block_id: Uuid,
    pub reverted_blocks: Vec<Block>,
    pub all_edge_ids: Vec<Uuid>,
    pub name_to_remove: String,
    pub event: BlockDeleted,
}
