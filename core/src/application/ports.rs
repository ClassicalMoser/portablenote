use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::types::{Block, Document, Edge, Manifest};

/// Source of current time. Injected so domain and use cases stay deterministic and testable.
/// The infra crate provides `SystemClock`; use mocks in unit tests.
#[cfg_attr(test, mockall::automock)]
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

/// Read-only access to the block heap.
///
/// Use cases query through this trait. Writes are expressed as `VaultWrite`
/// entries in `CommandResult` and applied by the adapter — never by use cases.
#[cfg_attr(test, mockall::automock)]
pub trait BlockStore {
    fn get(&self, id: Uuid) -> Option<Block>;
    fn list(&self) -> Vec<Block>;
    /// Return all blocks whose content contains `[[name]]`.
    fn find_by_ref(&self, name: &str) -> Vec<Block>;
}

/// Read-only access to the reference graph.
#[cfg_attr(test, mockall::automock)]
pub trait GraphStore {
    fn get_edge(&self, id: Uuid) -> Option<Edge>;
    fn incoming(&self, block_id: Uuid) -> Vec<Edge>;
    /// All edges where `source == block_id` OR `target == block_id`.
    fn edges_for(&self, block_id: Uuid) -> Vec<Edge>;
}

/// Read-only access to document definitions.
#[cfg_attr(test, mockall::automock)]
pub trait DocumentStore {
    fn get(&self, id: Uuid) -> Option<Document>;
}

/// Read-only name → UUID index (vault-wide unique).
#[cfg_attr(test, mockall::automock)]
pub trait NameIndex {
    fn resolve(&self, name: &str) -> Option<Uuid>;
    /// Case-insensitive lookup. Returns `Some((stored_name, uuid))` if any name matches.
    fn resolve_ignore_case(&self, name: &str) -> Option<(String, Uuid)>;
}

/// Read and write the vault manifest (checksum chain).
///
/// The manifest is the **commit snapshot**: each successful commit updates
/// `checksum` and `previous_checksum` per §5a. The adapter implements this
/// port and is responsible for the full commit protocol (journal → apply
/// writes → write manifest = commit point → delete journal) so that commits
/// are reconstructible and crash-safe. This port is the "save snapshot"
/// boundary in the hexagon.
#[cfg_attr(test, mockall::automock)]
pub trait ManifestStore {
    fn get(&self) -> Option<Manifest>;
    fn write(&self, manifest: &Manifest);
}

/// Mutation gate (§5): checksum check, then full validation on mismatch.
///
/// A single port so we don't extend other ports with "list everything" just
/// for the gate. The adapter builds the vault from its own state and calls
/// the core gate rule; the composition root calls this before permitting a
/// mutation.
#[cfg_attr(test, mockall::automock)]
pub trait MutationGate {
    /// Returns `Ok(())` if mutation is allowed, `Err(RemediationRequired)` if not.
    fn allow_mutation(&self) -> Result<(), crate::domain::error::DomainError>;
}

/// Injected port references for the composition root.
///
/// Built once (CLI, server, WASM adapter, or tests) and passed into use cases
/// so the hexagon has a single, mockable dependency boundary. Use
/// `UseCases::new(ports)` to get the use-case surface; use cases read through
/// these ports and return `CommandResult` for the adapter to apply.
///
/// The adapter is responsible for the §5a commit protocol: after applying
/// writes, it must update the manifest (via `manifest`) so the checksum chain
/// is persisted — the reconstructible atomic commit model.
#[derive(Clone, Copy)]
pub struct VaultPorts<'a> {
    pub blocks: &'a dyn BlockStore,
    pub graph: &'a dyn GraphStore,
    pub documents: &'a dyn DocumentStore,
    pub names: &'a dyn NameIndex,
    pub manifest: &'a dyn ManifestStore,
    pub clock: &'a dyn Clock,
}
