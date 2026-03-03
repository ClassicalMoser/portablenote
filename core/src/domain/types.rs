use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub name: String,
    pub content: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subsection {
    pub block: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub block: Uuid,
    pub subsections: Vec<Subsection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub root: Uuid,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub vault_id: Uuid,
    pub spec_version: String,
    pub format: String,
    pub checksum: String,
    pub names: HashMap<String, Uuid>,
}

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
    /// Monotonically increasing in-memory mutation counter.
    /// Bumped by every aggregate method. Reset to 0 on load.
    pub version: u64,
}
