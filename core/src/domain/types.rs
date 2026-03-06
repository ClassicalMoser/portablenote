use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The atomic unit of knowledge. A block is a named, content-bearing entity
/// stored as a single `.md` file in the vault's `blocks/` directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    /// Human-readable name — vault-wide unique, used in `[[wikilink]]` syntax.
    pub name: String,
    /// Markdown body. Must not contain heading syntax outside fenced code blocks.
    pub content: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

/// A directed, typed reference between two blocks in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

/// A depth-3 entry within a document section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subsection {
    pub block: Uuid,
}

/// A depth-2 entry in a document, containing zero or more subsections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub block: Uuid,
    pub subsections: Vec<Subsection>,
}

/// An ordered composition of blocks from the heap, forming a readable document.
/// The same block may appear in multiple documents. Documents never affect the
/// graph — they are views, not sources of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    /// The block whose name becomes the document title (heading level 1).
    pub root: Uuid,
    pub sections: Vec<Section>,
}

/// Top-level vault metadata stored in `manifest.json`. The checksum chain
/// (`previous_checksum` → `checksum`) records each committed state transition
/// and allows fork detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub vault_id: Uuid,
    pub spec_version: String,
    pub format: String,
    pub checksum: String,
    /// Checksum of the vault state before the most recent commit.
    /// `None` only for the genesis commit (vault init).
    pub previous_checksum: Option<String>,
}

/// The vault's explicit reference graph, stored in `block-graph.json`.
/// Edges are first-class artifacts — they are not derived from scanning content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockGraph {
    pub version: String,
    pub edges: Vec<Edge>,
}

/// Read-only snapshot of a fully loaded vault.
///
/// Used for full-state validation (`invariants::validate_vault`) and checksum
/// computation at open/import time. Not the unit of command execution — commands
/// operate on individual artifacts loaded through port traits.
#[derive(Debug, Clone)]
pub struct Vault {
    pub manifest: Manifest,
    pub blocks: HashMap<Uuid, Block>,
    pub graph: BlockGraph,
    pub documents: HashMap<Uuid, Document>,
    /// Name-to-UUID index. Peer artifact to the graph and documents,
    /// loaded from `names.json` rather than the manifest.
    pub names: HashMap<String, Uuid>,
    /// Monotonically increasing in-memory mutation counter.
    /// Bumped by every aggregate method. Reset to 0 on load.
    pub version: u64,
}
