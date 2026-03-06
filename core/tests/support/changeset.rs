#![allow(dead_code)] // shared test infra — not every binary uses every function

use portablenote_core::application::results::VaultWrite;

use super::in_memory::VaultStores;

/// Apply an ordered list of `VaultWrite` entries to in-memory stores.
///
/// This is the in-process equivalent of what a production adapter's atomic
/// commit would do. Order matters: e.g. reverted blocks must be saved before
/// the deleted block's name is removed.
pub fn apply_writes(stores: &mut VaultStores, writes: Vec<VaultWrite>) {
    for write in writes {
        match write {
            VaultWrite::WriteBlock(block) => stores.blocks.save(&block),
            VaultWrite::DeleteBlock(id) => stores.blocks.delete(id),
            VaultWrite::WriteEdge(edge) => stores.graph.save_edge(&edge),
            VaultWrite::RemoveEdge(id) => stores.graph.remove_edge(id),
            VaultWrite::WriteDocument(doc) => stores.documents.save(&doc),
            VaultWrite::DeleteDocument(id) => stores.documents.delete(id),
            VaultWrite::SetName { name, id } => stores.names.set(&name, id),
            VaultWrite::RemoveName(name) => stores.names.remove(&name),
        }
    }
}
