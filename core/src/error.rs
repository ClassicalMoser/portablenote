use uuid::Uuid;

/// A specific invariant violation detected during vault validation.
#[derive(Debug, Clone)]
pub struct Violation {
    pub description: String,
    pub details: ViolationDetails,
}

#[derive(Debug, Clone)]
pub enum ViolationDetails {
    DanglingEdgeUuid {
        edge_id: Uuid,
        dangling_uuid: Uuid,
        field: String,
    },
    DanglingDocumentUuid {
        document_id: Uuid,
        dangling_uuid: Uuid,
        field: String,
    },
    DocumentCycle {
        document_id: Uuid,
        block_id: Uuid,
    },
    InvalidEdgeEndpoint {
        edge_id: Uuid,
    },
    MissingFooterAnnotation {
        block_id: Uuid,
        referenced_name: String,
    },
    MissingEdgeForRef {
        block_id: Uuid,
        referenced_name: String,
        target_id: Uuid,
    },
    DanglingFooterAnnotation {
        block_id: Uuid,
        name: String,
    },
    DuplicateName {
        name: String,
        block_ids: Vec<Uuid>,
    },
    UuidMismatch {
        file_uuid: Uuid,
        metadata_uuid: Uuid,
    },
    HeadingInContent {
        block_id: Uuid,
        heading_text: String,
        heading_level: u8,
    },
    ChecksumMismatch {
        manifest_checksum: String,
        computed_checksum: String,
    },
    MissingMetadataField {
        block_id: Uuid,
        missing_field: String,
    },
}

/// Domain errors for command execution.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Block {0} not found")]
    BlockNotFound(Uuid),

    #[error("Document {0} not found")]
    DocumentNotFound(Uuid),

    #[error("Edge {0} not found")]
    EdgeNotFound(Uuid),

    #[error("Name '{0}' is already in use by block {1}")]
    NameConflict(String, Uuid),

    #[error("Block {0} has {1} incoming edge(s); use cascade to force deletion")]
    HasIncomingEdges(Uuid, usize),

    #[error("Block content contains heading syntax outside fenced code block")]
    HeadingInContent,

    #[error("Target block {0} does not exist in heap")]
    TargetNotInHeap(Uuid),

    #[error("Source block {0} does not exist in heap")]
    SourceNotInHeap(Uuid),

    #[error("Root block {0} does not exist in heap")]
    RootNotInHeap(Uuid),

    #[error("Section {0} not found in document")]
    SectionNotFound(Uuid),

    #[error("Save conflict: artifact modified since base_version {0}")]
    SaveConflict(u64),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}
