//! Commit boundary: update manifest after writes (§5a).
//!
//! The adapter applies writes, then calls this so the checksum chain is
//! persisted. Without it, the vault is inconsistent (computed checksum ≠ manifest).

use crate::application::ports::ManifestStore;
use crate::domain::checksum;
use crate::domain::types::Vault;

/// Persist the manifest with the current vault state as the new commit point.
/// Call after `apply_writes`. Updates `checksum` to the computed value and
/// sets `previous_checksum` to the prior `manifest.checksum`.
pub fn write_manifest_after_writes(vault: &Vault, store: &dyn ManifestStore) {
    let new_checksum = checksum::compute(vault);
    let mut manifest = vault.manifest.clone();
    manifest.previous_checksum = Some(manifest.checksum.clone());
    manifest.checksum = new_checksum;
    store.write(&manifest);
}
