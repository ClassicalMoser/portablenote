use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use portablenote_core::application::ports::DocumentStore;
use portablenote_core::domain::types::Document;

/// Filesystem adapter for `DocumentStore`. Manages the `documents/` directory
/// where each document is a JSON file named `<uuid>.json`. All documents are
/// loaded into memory on `open`; mutations write through to disk immediately.
pub struct FsDocumentStore {
    dir: PathBuf,
    cache: HashMap<Uuid, Document>,
}

impl FsDocumentStore {
    /// Load all document definitions from the given directory.
    /// Creates the directory if it does not exist.
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        let mut cache = HashMap::new();

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let raw = fs::read_to_string(&path)?;
            let doc: Document = serde_json::from_str(&raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            cache.insert(doc.id, doc);
        }

        Ok(Self { dir, cache })
    }

    /// All documents (for MutationGate implementation in this crate).
    pub fn all_documents(&self) -> &HashMap<Uuid, Document> {
        &self.cache
    }

    pub fn save(&mut self, doc: &Document) {
        self.cache.insert(doc.id, doc.clone());
        self.flush_one(doc).expect("failed to write document file");
    }

    pub fn delete(&mut self, id: Uuid) {
        self.cache.remove(&id);
        let path = self.doc_path(id);
        if path.exists() {
            fs::remove_file(path).expect("failed to delete document file");
        }
    }

    fn doc_path(&self, id: Uuid) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    fn flush_one(&self, doc: &Document) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(doc)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(self.doc_path(doc.id), json)
    }
}

impl DocumentStore for FsDocumentStore {
    fn get(&self, id: Uuid) -> Option<Document> {
        self.cache.get(&id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portablenote_core::domain::types::Section;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let docs_dir = dir.path().join("documents");
        (dir, docs_dir)
    }

    fn make_doc() -> Document {
        Document {
            id: Uuid::new_v4(),
            root: Uuid::new_v4(),
            sections: vec![Section { block: Uuid::new_v4(), subsections: vec![] }],
        }
    }

    #[test]
    fn open_creates_dir_when_missing() {
        let (_dir, docs_dir) = setup();
        assert!(!docs_dir.exists());
        FsDocumentStore::open(docs_dir.clone()).unwrap();
        assert!(docs_dir.exists());
    }

    #[test]
    fn save_and_get() {
        let (_dir, docs_dir) = setup();
        let mut store = FsDocumentStore::open(docs_dir).unwrap();
        let doc = make_doc();

        store.save(&doc);

        let fetched = store.get(doc.id).unwrap();
        assert_eq!(fetched.id, doc.id);
        assert_eq!(fetched.root, doc.root);
        assert_eq!(fetched.sections.len(), 1);
    }

    #[test]
    fn save_persists_to_disk() {
        let (_dir, docs_dir) = setup();
        let doc = make_doc();

        {
            let mut store = FsDocumentStore::open(docs_dir.clone()).unwrap();
            store.save(&doc);
        }

        let store = FsDocumentStore::open(docs_dir).unwrap();
        assert!(store.get(doc.id).is_some());
    }

    #[test]
    fn delete_removes_from_cache_and_disk() {
        let (_dir, docs_dir) = setup();
        let mut store = FsDocumentStore::open(docs_dir.clone()).unwrap();
        let doc = make_doc();
        let id = doc.id;

        store.save(&doc);
        assert!(store.get(id).is_some());

        store.delete(id);
        assert!(store.get(id).is_none());
        assert!(!docs_dir.join(format!("{id}.json")).exists());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (_dir, docs_dir) = setup();
        let store = FsDocumentStore::open(docs_dir).unwrap();
        assert!(store.get(Uuid::new_v4()).is_none());
    }
}
