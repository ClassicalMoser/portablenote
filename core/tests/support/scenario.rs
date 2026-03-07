//! Typed deserialization structs for the mutation scenario JSON format
//! used by the compliance test suite. Each JSON file in
//! `spec/compliance/mutations/` describes a single command against an
//! initial vault snapshot and the expected post-mutation assertions.

#![allow(dead_code)] // shared test infra — not every binary uses every type

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MutationScenario {
    pub description: String,
    pub initial_vault: String,
    pub command: ScenarioCommand,
    pub expected: Expected,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioCommand {
    /// Command type name, e.g. "AddBlock", "DeleteBlock".
    #[serde(rename = "type")]
    pub kind: String,
    /// Payload varies per command; kept as raw JSON for per-command extraction.
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct Expected {
    pub result: String,
    /// Present when `result == "rejected"`.
    pub error: Option<String>,
    /// Present when `result == "success"`.
    #[serde(default)]
    pub assertions: Vec<Assertion>,
}

/// Each variant corresponds to an assertion `type` value in the JSON.
/// Fields are named to match the JSON keys exactly.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Assertion {
    #[serde(rename = "block_exists")]
    BlockExists { block_id: String },

    #[serde(rename = "block_not_exists")]
    BlockNotExists { block_id: String },

    #[serde(rename = "block_content_is")]
    BlockContentIs { block_id: String, content: String },

    #[serde(rename = "block_content_contains")]
    BlockContentContains { block_id: String, text: String },

    #[serde(rename = "block_content_not_contains")]
    BlockContentNotContains { block_id: String, text: String },

    #[serde(rename = "block_footer_not_contains")]
    BlockFooterNotContains { block_id: String, name: String },

    #[serde(rename = "block_modified_after")]
    BlockModifiedAfter { block_id: String, after: String },

    #[serde(rename = "name_index_missing")]
    NameIndexMissing { name: String },

    #[serde(rename = "name_index_contains")]
    NameIndexContains { name: String, block_id: String },

    #[serde(rename = "block_count")]
    BlockCount { count: usize },

    #[serde(rename = "block_name_is")]
    BlockNameIs { block_id: String, name: String },

    #[serde(rename = "block_footer_maps")]
    BlockFooterMaps {
        block_id: String,
        name: String,
        target_uuid: String,
    },

    #[serde(rename = "edge_exists")]
    EdgeExists { edge_id: String },

    #[serde(rename = "edge_not_exists")]
    EdgeNotExists { edge_id: String },

    #[serde(rename = "edge_count")]
    EdgeCount { count: usize },

    #[serde(rename = "document_exists")]
    DocumentExists { document_id: String },

    #[serde(rename = "document_not_exists")]
    DocumentNotExists { document_id: String },

    #[serde(rename = "document_root_is")]
    DocumentRootIs { document_id: String, root: String },

    #[serde(rename = "document_section_count")]
    DocumentSectionCount { document_id: String, count: usize },

    #[serde(rename = "document_section_at")]
    DocumentSectionAt {
        document_id: String,
        index: usize,
        block_id: String,
    },

    #[serde(rename = "document_subsection_count")]
    DocumentSubsectionCount {
        document_id: String,
        section_block_id: String,
        count: usize,
    },

    #[serde(rename = "event_emitted")]
    EventEmitted {
        event: String,
        /// Remaining fields vary per event type — captured as raw extras.
        #[serde(flatten)]
        fields: serde_json::Map<String, serde_json::Value>,
    },
}
