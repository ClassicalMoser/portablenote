#![allow(dead_code)] // shared test infra — not every binary uses every function

use portablenote_core::application::ports::{BlockStore, GraphStore, NameIndex};
use portablenote_core::application::results::{
    AddBlockResult, DeleteBlockCascadeResult, DeleteBlockSafeResult, RenameBlockResult,
};

use super::in_memory::VaultStores;

/// Apply an AddBlockResult: persist the new block and register its name.
pub fn apply_add_block(stores: &mut VaultStores, result: AddBlockResult) {
    stores.blocks.save(&result.block);
    stores.names.set(&result.block.name, result.block.id);
}

/// Apply a RenameBlockResult: persist the renamed block and all propagated
/// blocks (with updated inline refs), swap the name index entry.
pub fn apply_rename_block(stores: &mut VaultStores, result: RenameBlockResult) {
    stores.blocks.save(&result.renamed);
    stores.blocks.save_all(&result.propagated);
    stores.names.remove(&result.old_name);
    stores.names.set(&result.renamed.name, result.renamed.id);
}

/// Apply a DeleteBlockSafeResult: persist reverted blocks, remove outgoing
/// edges, delete the block, and remove its name.
pub fn apply_delete_block_safe(stores: &mut VaultStores, result: DeleteBlockSafeResult) {
    stores.blocks.save_all(&result.reverted_blocks);
    stores.graph.remove_edges(&result.outgoing_edge_ids);
    stores.blocks.delete(result.block_id);
    stores.names.remove(&result.name_to_remove);
}

/// Apply a DeleteBlockCascadeResult: persist reverted blocks, remove all
/// edges (incoming + outgoing), delete the block, and remove its name.
pub fn apply_delete_block_cascade(stores: &mut VaultStores, result: DeleteBlockCascadeResult) {
    stores.blocks.save_all(&result.reverted_blocks);
    stores.graph.remove_edges(&result.all_edge_ids);
    stores.blocks.delete(result.block_id);
    stores.names.remove(&result.name_to_remove);
}
