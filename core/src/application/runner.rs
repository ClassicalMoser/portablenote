//! Use-case surface with injected ports (composition-root boundary).
//!
//! Build `VaultPorts` from your adapters (fs, in-memory, or mocks), then
//! `UseCases::new(ports)`. All use-case calls go through this struct so the
//! hexagon has a single, mockable dependency boundary — same pattern as
//! `createUseCases({ databasePort, authPort, ... })` in a TS/Express bootstrap.

use uuid::Uuid;

use crate::application::ports::VaultPorts;
use crate::application::results::CommandResult;
use crate::domain::error::DomainError;
use crate::domain::events::{
    BlockAdded, BlockContentMutated, BlockDeleted, BlockRenamed, DocumentAdded, DocumentDeleted,
    EdgeAdded, EdgeRemoved, SectionAppended, SectionRemoved, SectionsReordered,
};

use crate::application::use_cases::{
    add_block, add_document, add_edge, append_section, append_subsection, delete_block_cascade,
    delete_block_safe, delete_document, mutate_block_content, remove_edge, remove_section,
    rename_block, reorder_sections,
};

/// Use-case surface with ports injected. Create via `UseCases::new(ports)` at
/// the composition root; pass to routes or call directly.
#[derive(Clone, Copy)]
pub struct UseCases<'a> {
    ports: VaultPorts<'a>,
}

impl<'a> UseCases<'a> {
    pub fn new(ports: VaultPorts<'a>) -> Self {
        Self { ports }
    }

    pub fn add_block(
        &self,
        id: Uuid,
        name: &str,
        content: &str,
    ) -> Result<CommandResult<BlockAdded>, DomainError> {
        add_block::execute(
            self.ports.blocks,
            self.ports.names,
            self.ports.clock,
            id,
            name,
            content,
        )
    }

    pub fn rename_block(
        &self,
        block_id: Uuid,
        new_name: &str,
    ) -> Result<CommandResult<BlockRenamed>, DomainError> {
        rename_block::execute(
            self.ports.blocks,
            self.ports.names,
            self.ports.clock,
            block_id,
            new_name,
        )
    }

    pub fn mutate_block_content(
        &self,
        block_id: Uuid,
        content: &str,
    ) -> Result<CommandResult<BlockContentMutated>, DomainError> {
        mutate_block_content::execute(self.ports.blocks, self.ports.clock, block_id, content)
    }

    pub fn delete_block_safe(
        &self,
        block_id: Uuid,
    ) -> Result<CommandResult<BlockDeleted>, DomainError> {
        delete_block_safe::execute(
            self.ports.blocks,
            self.ports.graph,
            self.ports.clock,
            block_id,
        )
    }

    pub fn delete_block_cascade(
        &self,
        block_id: Uuid,
    ) -> Result<CommandResult<BlockDeleted>, DomainError> {
        delete_block_cascade::execute(
            self.ports.blocks,
            self.ports.graph,
            self.ports.documents,
            self.ports.clock,
            block_id,
        )
    }

    pub fn add_edge(
        &self,
        id: Uuid,
        source: Uuid,
        target: Uuid,
    ) -> Result<CommandResult<EdgeAdded>, DomainError> {
        add_edge::execute(self.ports.blocks, self.ports.graph, id, source, target)
    }

    pub fn remove_edge(&self, edge_id: Uuid) -> Result<CommandResult<EdgeRemoved>, DomainError> {
        remove_edge::execute(self.ports.graph, edge_id)
    }

    pub fn add_document(
        &self,
        id: Uuid,
        root: Uuid,
    ) -> Result<CommandResult<DocumentAdded>, DomainError> {
        add_document::execute(self.ports.blocks, self.ports.documents, id, root)
    }

    pub fn delete_document(
        &self,
        document_id: Uuid,
    ) -> Result<CommandResult<DocumentDeleted>, DomainError> {
        delete_document::execute(self.ports.documents, document_id)
    }

    pub fn append_section(
        &self,
        document_id: Uuid,
        block_id: Uuid,
    ) -> Result<CommandResult<SectionAppended>, DomainError> {
        append_section::execute(
            self.ports.blocks,
            self.ports.documents,
            document_id,
            block_id,
        )
    }

    pub fn append_subsection(
        &self,
        document_id: Uuid,
        section_block_id: Uuid,
        block_id: Uuid,
    ) -> Result<CommandResult<SectionAppended>, DomainError> {
        append_subsection::execute(
            self.ports.blocks,
            self.ports.documents,
            document_id,
            section_block_id,
            block_id,
        )
    }

    pub fn remove_section(
        &self,
        document_id: Uuid,
        block_id: Uuid,
    ) -> Result<CommandResult<SectionRemoved>, DomainError> {
        remove_section::execute(self.ports.documents, document_id, block_id)
    }

    pub fn reorder_sections(
        &self,
        document_id: Uuid,
        section_order: Vec<Uuid>,
    ) -> Result<CommandResult<SectionsReordered>, DomainError> {
        reorder_sections::execute(self.ports.documents, document_id, section_order)
    }
}
