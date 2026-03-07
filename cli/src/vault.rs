//! CLI composition root: real filesystem integration.
//!
//! This module is the only place that wires concrete adapters (`FsBlockStore`,
//! `FsGraphStore`, etc.) into the hexagon. It builds `VaultPorts` from the
//! session and runs use cases through `UseCases::new(ports)`; commands then
//! apply the returned writes via `apply_writes`. Same DI pattern as a
//! `createHexagonRoutes()` / `createUseCases({ databasePort, authPort })`
//! bootstrap elsewhere.

use std::io;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use portablenote_core::application::ports::{BlockStore, MutationGate, VaultPorts};
use portablenote_infra::SystemClock;
use portablenote_core::application::results::VaultWrite;
use portablenote_core::application::runner::UseCases;
use portablenote_core::domain::error::DomainError;
use portablenote_infra::fs::{
    FsBlockStore, FsDocumentStore, FsGraphStore, FsManifestStore, FsMutationGate, FsNameIndex,
};

/// Open vault session: owns the real FS adapters and exposes the use-case
/// surface via injected `VaultPorts`. Includes `ManifestStore` for the
/// reconstructible atomic commit model (§5a).
pub struct VaultSession {
    pub blocks: FsBlockStore,
    pub graph: FsGraphStore,
    pub documents: FsDocumentStore,
    pub names: FsNameIndex,
    pub manifest: FsManifestStore,
    clock: SystemClock,
}

impl VaultSession {
    /// Initialize a new empty vault at the given path.
    pub fn init(vault_path: &Path) -> io::Result<()> {
        let pn = vault_path.join("portablenote");
        std::fs::create_dir_all(pn.join("blocks"))?;
        std::fs::create_dir_all(pn.join("documents"))?;

        let manifest = serde_json::json!({
            "vault_id": Uuid::new_v4().to_string(),
            "spec_version": "0.1.0",
            "format": "markdown",
            "checksum": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "previous_checksum": null
        });
        std::fs::write(
            pn.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )?;

        std::fs::write(
            pn.join("block-graph.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "version": "0.1.0",
                "edges": []
            }))
            .unwrap(),
        )?;

        std::fs::write(pn.join("names.json"), "{}")?;

        Ok(())
    }

    /// Open an existing vault at the given path.
    pub fn open(vault_path: &Path) -> io::Result<Self> {
        let pn = vault_path.join("portablenote");
        if !pn.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no portablenote directory at {}", vault_path.display()),
            ));
        }

        Ok(Self {
            blocks: FsBlockStore::open(pn.join("blocks"))?,
            graph: FsGraphStore::open(pn.join("block-graph.json"))?,
            documents: FsDocumentStore::open(pn.join("documents"))?,
            names: FsNameIndex::open(pn.join("names.json"))?,
            manifest: FsManifestStore::open(pn.join("manifest.json")),
            clock: SystemClock,
        })
    }

    /// Resolve vault path: use the provided path or search upward from cwd.
    pub fn resolve_vault_path(explicit: Option<&Path>) -> io::Result<PathBuf> {
        if let Some(p) = explicit {
            return Ok(p.to_path_buf());
        }
        let mut dir = std::env::current_dir()?;
        loop {
            if dir.join("portablenote").exists() {
                return Ok(dir);
            }
            if !dir.pop() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "no portablenote vault found in current directory or ancestors",
                ));
            }
        }
    }

    /// Apply an ordered list of `VaultWrite` entries to the concrete adapters.
    ///
    /// This is the boundary at which the pure domain result becomes an
    /// impure filesystem operation. A future enhancement can make this
    /// atomic (stage → rename) to avoid partial writes.
    pub fn apply_writes(&mut self, writes: Vec<VaultWrite>) {
        for write in writes {
            match write {
                VaultWrite::WriteBlock(block) => self.blocks.save(&block),
                VaultWrite::DeleteBlock(id) => self.blocks.delete(id),
                VaultWrite::WriteEdge(edge) => self.graph.save_edge(&edge),
                VaultWrite::RemoveEdge(id) => self.graph.remove_edge(id),
                VaultWrite::WriteDocument(doc) => self.documents.save(&doc),
                VaultWrite::DeleteDocument(id) => self.documents.delete(id),
                VaultWrite::SetName { name, id } => self.names.set(&name, id),
                VaultWrite::RemoveName(name) => self.names.remove(&name),
            }
        }
    }

    /// Injected ports for the use-case layer (composition root).
    fn ports(&self) -> VaultPorts<'_> {
        VaultPorts {
            blocks: &self.blocks,
            graph: &self.graph,
            documents: &self.documents,
            names: &self.names,
            manifest: &self.manifest,
            clock: &self.clock,
        }
    }

    /// Use-case surface with ports injected. Borrows `self` for reads only.
    fn use_cases(&self) -> UseCases<'_> {
        UseCases::new(self.ports())
    }

    /// Mutation gate (§5): run before every mutating command.
    fn require_gate(&self) -> Result<(), DomainError> {
        let gate = FsMutationGate {
            blocks: &self.blocks,
            graph: &self.graph,
            documents: &self.documents,
            names: &self.names,
            manifest: &self.manifest,
        };
        gate.allow_mutation()
    }

    pub fn add_block(&mut self, name: &str, content: &str) -> Result<(), DomainError> {
        self.require_gate()?;
        let id = Uuid::new_v4();
        let result = self.use_cases().add_block(id, name, content)?;
        println!("added block: {} ({})", result.event.name, result.event.block_id);
        self.apply_writes(result.writes);
        Ok(())
    }

    pub fn rename_block(&mut self, block_id: Uuid, new_name: &str) -> Result<(), DomainError> {
        self.require_gate()?;
        let result = self.use_cases().rename_block(block_id, new_name)?;
        println!(
            "renamed block {} → {} ({} refs updated)",
            result.event.old_name, result.event.new_name, result.event.refs_updated
        );
        self.apply_writes(result.writes);
        Ok(())
    }

    pub fn mutate_content(&mut self, block_id: Uuid, content: &str) -> Result<(), DomainError> {
        self.require_gate()?;
        let result = self.use_cases().mutate_block_content(block_id, content)?;
        self.apply_writes(result.writes);
        println!("updated content for block {block_id}");
        Ok(())
    }

    pub fn delete_block(&mut self, block_id: Uuid, cascade: bool) -> Result<(), DomainError> {
        self.require_gate()?;
        if cascade {
            let result = self.use_cases().delete_block_cascade(block_id)?;
            println!(
                "deleted block {} (cascade: {} edges removed, {} refs reverted)",
                block_id, result.event.edges_removed, result.event.inline_refs_reverted
            );
            self.apply_writes(result.writes);
        } else {
            let result = self.use_cases().delete_block_safe(block_id)?;
            println!("deleted block {block_id}");
            self.apply_writes(result.writes);
        }
        Ok(())
    }

    pub fn add_edge(&mut self, source: Uuid, target: Uuid) -> Result<(), DomainError> {
        self.require_gate()?;
        let id = Uuid::new_v4();
        let result = self.use_cases().add_edge(id, source, target)?;
        println!("added edge {}: {} → {}", result.event.edge_id, result.event.source, result.event.target);
        self.apply_writes(result.writes);
        Ok(())
    }

    pub fn remove_edge(&mut self, edge_id: Uuid) -> Result<(), DomainError> {
        self.require_gate()?;
        let result = self.use_cases().remove_edge(edge_id)?;
        println!("removed edge {edge_id}");
        self.apply_writes(result.writes);
        Ok(())
    }

    pub fn list_blocks(&self) {
        let mut blocks = self.blocks.list();
        blocks.sort_by(|a, b| a.name.cmp(&b.name));

        if blocks.is_empty() {
            println!("no blocks in vault");
            return;
        }

        for block in &blocks {
            let preview: String = block
                .content
                .chars()
                .take(60)
                .collect::<String>()
                .replace('\n', " ");
            println!("  {} {} {}", block.id, block.name, preview);
        }
        println!("{} block(s)", blocks.len());
    }
}
