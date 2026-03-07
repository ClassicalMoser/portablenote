//! Filesystem-backed implementations of the core port traits.
//!
//! All adapters follow the same pattern: `open` loads the on-disk state into
//! an in-memory cache, mutations update both the cache and disk synchronously
//! (write-through). This keeps reads cheap while ensuring persistence after
//! every command.
//!
//! ## Vault directory layout
//!
//! ```text
//! portablenote/
//!   manifest.json          ← vault identity and checksum
//!   block-graph.json       ← FsGraphStore
//!   names.json             ← FsNameIndex
//!   blocks/                ← FsBlockStore (one .md per block)
//!   documents/             ← FsDocumentStore (one .json per document)
//! ```

mod block_store;
mod document_store;
mod encoding;
mod graph_store;
mod manifest_store;
mod mutation_gate;
mod name_index;

pub use block_store::FsBlockStore;
pub use document_store::FsDocumentStore;
pub use encoding::{decode_block_filename, encode_block_filename};
pub use graph_store::FsGraphStore;
pub use manifest_store::FsManifestStore;
pub use mutation_gate::FsMutationGate;
pub use name_index::FsNameIndex;
