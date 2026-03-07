use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use portablenote_core::application::ports::NameIndex;

/// Filesystem adapter for `NameIndex`. Manages `names.json` as a
/// flat `{ "name": "uuid" }` map. The full index is held in memory;
/// mutations flush the entire file on each write.
pub struct FsNameIndex {
    path: PathBuf,
    names: HashMap<String, Uuid>,
}

impl FsNameIndex {
    /// Load the name index from `names.json` at the given path.
    /// Creates an empty index if the file does not exist.
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let names = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            let raw_map: HashMap<String, String> = serde_json::from_str(&raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            raw_map
                .into_iter()
                .filter_map(|(name, id_str)| {
                    Uuid::parse_str(&id_str).ok().map(|id| (name, id))
                })
                .collect()
        } else {
            HashMap::new()
        };

        Ok(Self { path, names })
    }

    /// Full name→UUID map (for MutationGate implementation in this crate).
    pub fn all_names(&self) -> &HashMap<String, Uuid> {
        &self.names
    }

    pub fn set(&mut self, name: &str, id: Uuid) {
        self.names.insert(name.to_string(), id);
        self.flush().expect("failed to write names.json");
    }

    pub fn remove(&mut self, name: &str) {
        self.names.remove(name);
        self.flush().expect("failed to write names.json");
    }

    fn flush(&self) -> std::io::Result<()> {
        let raw_map: HashMap<&String, String> =
            self.names.iter().map(|(k, v)| (k, v.to_string())).collect();
        let json = serde_json::to_string_pretty(&raw_map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json)
    }
}

impl NameIndex for FsNameIndex {
    fn resolve(&self, name: &str) -> Option<Uuid> {
        self.names.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("names.json");
        (dir, path)
    }

    #[test]
    fn open_empty_when_file_missing() {
        let (_dir, path) = setup();
        let index = FsNameIndex::open(path).unwrap();
        assert!(index.resolve("Anything").is_none());
    }

    #[test]
    fn set_and_resolve() {
        let (_dir, path) = setup();
        let mut index = FsNameIndex::open(path).unwrap();
        let id = Uuid::new_v4();

        index.set("Alpha", id);
        assert_eq!(index.resolve("Alpha"), Some(id));
    }

    #[test]
    fn set_persists_round_trip() {
        let (_dir, path) = setup();
        let id = Uuid::new_v4();

        {
            let mut index = FsNameIndex::open(path.clone()).unwrap();
            index.set("Alpha", id);
        }

        let index = FsNameIndex::open(path).unwrap();
        assert_eq!(index.resolve("Alpha"), Some(id));
    }

    #[test]
    fn remove_deletes_entry() {
        let (_dir, path) = setup();
        let mut index = FsNameIndex::open(path).unwrap();
        let id = Uuid::new_v4();

        index.set("Alpha", id);
        index.remove("Alpha");

        assert!(index.resolve("Alpha").is_none());
    }

    #[test]
    fn remove_nonexistent_does_not_panic() {
        let (_dir, path) = setup();
        let mut index = FsNameIndex::open(path).unwrap();
        index.remove("DoesNotExist");
    }

    #[test]
    fn set_overwrites_existing() {
        let (_dir, path) = setup();
        let mut index = FsNameIndex::open(path).unwrap();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        index.set("Alpha", id1);
        index.set("Alpha", id2);

        assert_eq!(index.resolve("Alpha"), Some(id2));
    }
}
