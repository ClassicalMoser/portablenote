use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use portablenote_core::application::ports::GraphStore;
use portablenote_core::domain::types::{BlockGraph, Edge};

/// Filesystem adapter for `GraphStore`. Manages `block-graph.json` as a
/// single JSON file containing all edges. The full graph is held in memory;
/// mutations flush the entire file on each write.
pub struct FsGraphStore {
    path: PathBuf,
    graph: BlockGraph,
}

impl FsGraphStore {
    /// Full graph snapshot (for MutationGate implementation in this crate).
    pub fn as_block_graph(&self) -> &BlockGraph {
        &self.graph
    }

    /// Load the block graph from `block-graph.json` at the given path.
    /// Creates an empty graph if the file does not exist.
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let graph = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            serde_json::from_str(&raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            BlockGraph {
                version: "0.1.0".to_string(),
                edges: Vec::new(),
            }
        };
        Ok(Self { path, graph })
    }

    pub fn save_edge(&mut self, edge: &Edge) {
        self.graph.edges.retain(|e| e.id != edge.id);
        self.graph.edges.push(edge.clone());
        self.flush().expect("failed to write block-graph.json");
    }

    pub fn remove_edge(&mut self, id: Uuid) {
        self.graph.edges.retain(|e| e.id != id);
        self.flush().expect("failed to write block-graph.json");
    }

    fn flush(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self.graph)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json)
    }
}

impl GraphStore for FsGraphStore {
    fn get_edge(&self, id: Uuid) -> Option<Edge> {
        self.graph.edges.iter().find(|e| e.id == id).cloned()
    }

    fn incoming(&self, block_id: Uuid) -> Vec<Edge> {
        self.graph
            .edges
            .iter()
            .filter(|e| e.target == block_id)
            .cloned()
            .collect()
    }

    fn edges_for(&self, block_id: Uuid) -> Vec<Edge> {
        self.graph
            .edges
            .iter()
            .filter(|e| e.source == block_id || e.target == block_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("block-graph.json");
        (dir, path)
    }

    fn edge(source: Uuid, target: Uuid) -> Edge {
        Edge { id: Uuid::new_v4(), source, target }
    }

    #[test]
    fn open_creates_empty_when_file_missing() {
        let (_dir, path) = setup();
        let store = FsGraphStore::open(path).unwrap();
        assert!(store.get_edge(Uuid::new_v4()).is_none());
    }

    #[test]
    fn save_edge_and_get() {
        let (_dir, path) = setup();
        let mut store = FsGraphStore::open(path).unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let e = edge(a, b);
        let eid = e.id;

        store.save_edge(&e);

        let fetched = store.get_edge(eid).unwrap();
        assert_eq!(fetched.source, a);
        assert_eq!(fetched.target, b);
    }

    #[test]
    fn save_edge_persists_to_disk() {
        let (_dir, path) = setup();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let e = edge(a, b);
        let eid = e.id;

        {
            let mut store = FsGraphStore::open(path.clone()).unwrap();
            store.save_edge(&e);
        }

        let store = FsGraphStore::open(path).unwrap();
        assert!(store.get_edge(eid).is_some());
    }

    #[test]
    fn incoming_filters_by_target() {
        let (_dir, path) = setup();
        let mut store = FsGraphStore::open(path).unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        store.save_edge(&edge(a, b));
        store.save_edge(&edge(c, b));
        store.save_edge(&edge(a, c));

        assert_eq!(store.incoming(b).len(), 2);
        assert_eq!(store.incoming(c).len(), 1);
        assert_eq!(store.incoming(a).len(), 0);
    }

    #[test]
    fn edges_for_includes_both_directions() {
        let (_dir, path) = setup();
        let mut store = FsGraphStore::open(path).unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        store.save_edge(&edge(a, b));
        store.save_edge(&edge(c, a));

        assert_eq!(store.edges_for(a).len(), 2);
    }

    #[test]
    fn remove_edge_deletes_by_id() {
        let (_dir, path) = setup();
        let mut store = FsGraphStore::open(path).unwrap();
        let e1 = edge(Uuid::new_v4(), Uuid::new_v4());
        let e2 = edge(Uuid::new_v4(), Uuid::new_v4());
        let e1_id = e1.id;
        let e2_id = e2.id;

        store.save_edge(&e1);
        store.save_edge(&e2);
        store.remove_edge(e1_id);

        assert!(store.get_edge(e1_id).is_none());
        assert!(store.get_edge(e2_id).is_some());
    }
}
