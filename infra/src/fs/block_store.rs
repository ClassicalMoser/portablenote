use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use portablenote_core::application::ports::BlockStore;
use portablenote_core::domain::format;
use portablenote_core::domain::types::Block;

use super::encoding::encode_block_filename;

/// Filesystem adapter for `BlockStore`. Manages the `blocks/` directory where
/// each block is a single `.md` file named by its percent-encoded block name.
///
/// On `open`, all `.md` files are parsed into an in-memory cache. Mutations
/// write through to disk immediately. Renames delete the old file and create
/// a new one to keep the filename in sync with `block.name`.
pub struct FsBlockStore {
    dir: PathBuf,
    cache: HashMap<Uuid, Block>,
}

impl FsBlockStore {
    /// Load all blocks from `.md` files in the given directory.
    /// Creates the directory if it does not exist.
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        let mut cache = HashMap::new();

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let raw = fs::read_to_string(&path)?;
            match format::parse_block_file(&raw) {
                Ok(block) => {
                    cache.insert(block.id, block);
                }
                Err(e) => {
                    eprintln!(
                        "warning: skipping malformed block file {}: {e}",
                        path.display()
                    );
                }
            }
        }

        Ok(Self { dir, cache })
    }

    pub fn save(&mut self, block: &Block) {
        if let Some(old) = self.cache.get(&block.id) {
            if old.name != block.name {
                self.remove_file(&old.name);
            }
        }
        self.cache.insert(block.id, block.clone());
        self.flush_one(block).expect("failed to write block file");
    }

    pub fn save_all(&mut self, blocks: &[Block]) {
        for block in blocks {
            if let Some(old) = self.cache.get(&block.id) {
                if old.name != block.name {
                    self.remove_file(&old.name);
                }
            }
            self.cache.insert(block.id, block.clone());
            self.flush_one(block).expect("failed to write block file");
        }
    }

    pub fn delete(&mut self, id: Uuid) {
        if let Some(block) = self.cache.remove(&id) {
            self.remove_file(&block.name);
        }
    }

    fn block_path(&self, name: &str) -> PathBuf {
        let filename = encode_block_filename(name);
        self.dir.join(format!("{filename}.md"))
    }

    fn flush_one(&self, block: &Block) -> std::io::Result<()> {
        let content = format::serialize_block_file(block);
        fs::write(self.block_path(&block.name), content)
    }

    fn remove_file(&self, name: &str) {
        let path = self.block_path(name);
        if path.exists() {
            fs::remove_file(path).expect("failed to delete block file");
        }
    }
}

impl BlockStore for FsBlockStore {
    fn get(&self, id: Uuid) -> Option<Block> {
        self.cache.get(&id).cloned()
    }

    fn list(&self) -> Vec<Block> {
        self.cache.values().cloned().collect()
    }

    fn find_by_ref(&self, name: &str) -> Vec<Block> {
        let pattern = format!("[[{name}]]");
        self.cache
            .values()
            .filter(|b| b.content.contains(&pattern))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let blocks_dir = dir.path().join("blocks");
        (dir, blocks_dir)
    }

    fn make_block(name: &str, content: &str) -> Block {
        let now = Utc::now();
        Block {
            id: Uuid::new_v4(),
            name: name.to_string(),
            content: content.to_string(),
            created: now,
            modified: now,
        }
    }

    #[test]
    fn open_creates_dir_when_missing() {
        let (_dir, blocks_dir) = setup();
        assert!(!blocks_dir.exists());
        FsBlockStore::open(blocks_dir.clone()).unwrap();
        assert!(blocks_dir.exists());
    }

    #[test]
    fn save_and_get() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir).unwrap();
        let block = make_block("Alpha", "Some content.");

        store.save(&block);

        let fetched = store.get(block.id).unwrap();
        assert_eq!(fetched.name, "Alpha");
        assert_eq!(fetched.content, "Some content.");
    }

    #[test]
    fn save_creates_md_file_on_disk() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir.clone()).unwrap();
        let block = make_block("My Block", "Content here.");

        store.save(&block);

        assert!(blocks_dir.join("My Block.md").exists());
    }

    #[test]
    fn save_persists_round_trip() {
        let (_dir, blocks_dir) = setup();
        let block = make_block("Alpha", "Some content.");
        let id = block.id;

        {
            let mut store = FsBlockStore::open(blocks_dir.clone()).unwrap();
            store.save(&block);
        }

        let store = FsBlockStore::open(blocks_dir).unwrap();
        let fetched = store.get(id).unwrap();
        assert_eq!(fetched.name, "Alpha");
        assert_eq!(fetched.content, "Some content.");
    }

    #[test]
    fn save_renames_file_when_name_changes() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir.clone()).unwrap();
        let mut block = make_block("OldName", "Content.");

        store.save(&block);
        assert!(blocks_dir.join("OldName.md").exists());

        block.name = "NewName".to_string();
        store.save(&block);

        assert!(!blocks_dir.join("OldName.md").exists());
        assert!(blocks_dir.join("NewName.md").exists());
    }

    #[test]
    fn list_returns_all_blocks() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir).unwrap();

        store.save(&make_block("A", "a"));
        store.save(&make_block("B", "b"));
        store.save(&make_block("C", "c"));

        assert_eq!(store.list().len(), 3);
    }

    #[test]
    fn find_by_ref_matches_inline_refs() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir).unwrap();

        store.save(&make_block("Referrer", "See [[Target]] here."));
        store.save(&make_block("Target", "I am the target."));
        store.save(&make_block("Other", "No refs here."));

        let results = store.find_by_ref("Target");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Referrer");
    }

    #[test]
    fn delete_removes_from_cache_and_disk() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir.clone()).unwrap();
        let block = make_block("Doomed", "Content.");
        let id = block.id;

        store.save(&block);
        assert!(blocks_dir.join("Doomed.md").exists());

        store.delete(id);
        assert!(store.get(id).is_none());
        assert!(!blocks_dir.join("Doomed.md").exists());
    }

    #[test]
    fn save_all_persists_multiple() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir.clone()).unwrap();
        let blocks = vec![make_block("X", "x"), make_block("Y", "y")];

        store.save_all(&blocks);

        assert!(blocks_dir.join("X.md").exists());
        assert!(blocks_dir.join("Y.md").exists());
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn filename_with_special_chars() {
        let (_dir, blocks_dir) = setup();
        let mut store = FsBlockStore::open(blocks_dir.clone()).unwrap();
        let block = make_block("Notes: Part 1", "Content.");

        store.save(&block);

        assert!(blocks_dir.join("Notes%3A Part 1.md").exists());
        assert_eq!(store.get(block.id).unwrap().name, "Notes: Part 1");
    }
}
