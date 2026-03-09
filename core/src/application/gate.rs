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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{BlockGraph, Manifest};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn minimal_vault(checksum: &str) -> Vault {
        Vault {
            manifest: Manifest {
                vault_id: Uuid::new_v4(),
                spec_version: "0.1.0".to_string(),
                format: "markdown".to_string(),
                checksum: checksum.to_string(),
                previous_checksum: None,
            },
            blocks: HashMap::new(),
            graph: BlockGraph { version: "0.1.0".to_string(), edges: vec![] },
            documents: HashMap::new(),
            names: HashMap::new(),
            block_refs: HashMap::new(),
            version: 0,
        }
    }

    #[test]
    fn allow_mutation_with_matching_expected_checksum() {
        let vault = minimal_vault("sha256:abc");
        assert!(mutation_gate(&vault, Some("sha256:abc")).is_ok());
    }

    #[test]
    fn allow_mutation_returns_stale_state_when_expected_mismatches() {
        let vault = minimal_vault("sha256:actual");
        let err = mutation_gate(&vault, Some("sha256:stale")).unwrap_err();
        match &err {
            DomainError::StaleState { expected, actual } => {
                assert_eq!(expected, "sha256:stale");
                assert_eq!(actual, "sha256:actual");
            }
            _ => panic!("expected StaleState, got {:?}", err),
        }
    }
}
