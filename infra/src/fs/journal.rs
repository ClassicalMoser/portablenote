//! Journal file I/O for §5a. Atomic write (temp + rename); read and delete for recovery.

use std::fs;
use std::io;
use std::path::Path;

use portablenote_core::application::journal::Journal;

const JOURNAL_FILENAME: &str = ".journal";
const JOURNAL_TEMP_SUFFIX: &str = ".journal.tmp";

/// Writes the journal atomically (temp file + rename). Reads and deletes for recovery.
pub struct FsJournalStore {
    path: std::path::PathBuf,
    temp_path: std::path::PathBuf,
}

impl FsJournalStore {
    /// Path is the portablenote directory; the journal is written as `path/.journal`.
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let temp_path = path.join(JOURNAL_TEMP_SUFFIX);
        let path = path.join(JOURNAL_FILENAME);
        Self { path, temp_path }
    }

    /// Returns true if `.journal` exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Write journal atomically (create temp, then rename).
    pub fn write(&self, journal: &Journal) -> io::Result<()> {
        let json = serde_json::to_string_pretty(journal)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&self.temp_path, json)?;
        fs::rename(&self.temp_path, &self.path)
    }

    /// Read journal if present.
    pub fn read(&self) -> io::Result<Option<Journal>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.path)?;
        let journal: Journal = serde_json::from_str(&raw)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(journal))
    }

    /// Remove the journal file. Idempotent.
    pub fn delete(&self) -> io::Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        if self.temp_path.exists() {
            let _ = fs::remove_file(&self.temp_path);
        }
        Ok(())
    }
}
