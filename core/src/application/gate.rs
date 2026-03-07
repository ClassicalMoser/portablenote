//! Mutation gate (§5): checksum check, then full validation on mismatch.
//!
//! Pure function so the rule lives in the core. Adapters build a `Vault` from
//! their state and call this; the `MutationGate` port wraps that.
//!
//! Optional optimistic concurrency: when `expected_checksum` is set, the gate
//! allows only if `vault.manifest.checksum == expected_checksum` (client's
//! read-state must match). The manifest's `previous_checksum` records the
//! prior committed state; clients send the `checksum` they last read.

use crate::domain::checksum;
use crate::domain::error::DomainError;
use crate::domain::invariants;
use crate::domain::types::Vault;

/// Enforces the mutation gate: optional OCC check → drift check → revalidate on mismatch;
/// violations → block with RemediationRequired; stale state → StaleState.
pub fn mutation_gate(vault: &Vault, expected_checksum: Option<&str>) -> Result<(), DomainError> {
    if let Some(exp) = expected_checksum {
        if vault.manifest.checksum != exp {
            return Err(DomainError::StaleState {
                expected: exp.to_string(),
                actual: vault.manifest.checksum.clone(),
            });
        }
    }
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
