//! Mutation gate (§5): checksum check, then full validation on mismatch.
//!
//! Pure function so the rule lives in the core. Adapters build a `Vault` from
//! their state and call this; the `MutationGate` port wraps that.

use crate::domain::checksum;
use crate::domain::error::DomainError;
use crate::domain::invariants;
use crate::domain::types::Vault;

/// Enforces the mutation gate: checksums match → allow; mismatch → revalidate;
/// violations → block with RemediationRequired.
pub fn mutation_gate(vault: &Vault) -> Result<(), DomainError> {
    if !checksum::is_drifted(vault) {
        return Ok(());
    }
    let violations = invariants::validate_vault(vault);
    if violations.is_empty() {
        Ok(())
    } else {
        Err(DomainError::RemediationRequired(violations.len()))
    }
}
