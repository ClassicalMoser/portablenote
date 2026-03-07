//! Filesystem adapter for `ManifestStore`. Reads and writes `manifest.json`.
//!
//! Used by the composition root to persist the checksum chain after each
//! commit (§5a). No in-memory cache: `get()` reads from disk, `write()` writes.

use std::fs;
use std::path::PathBuf;

use portablenote_core::application::ports::ManifestStore;
use portablenote_core::domain::types::Manifest;

/// Filesystem-backed manifest (manifest.json).
pub struct FsManifestStore {
    path: PathBuf,
}

impl FsManifestStore {
    /// Use the given path as manifest.json. Does not read until `get()`.
    pub fn open(path: PathBuf) -> Self {
        Self { path }
    }

    fn read(&self) -> std::io::Result<Option<Manifest>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.path)?;
        let manifest: Manifest = serde_json::from_str(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(manifest))
    }

    fn persist(&self, manifest: &Manifest) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json)
    }
}

impl ManifestStore for FsManifestStore {
    fn get(&self) -> Option<Manifest> {
        self.read().expect("failed to read manifest.json")
    }

    fn write(&self, manifest: &Manifest) {
        self.persist(manifest).expect("failed to write manifest.json");
    }
}
