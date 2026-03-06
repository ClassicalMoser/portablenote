//! Schema compliance tests: Rust-serialized artifact types must validate against
//! the spec's JSON schemas. Prevents spec/implementation drift.

use std::path::Path;

use chrono::Utc;
use jsonschema::JSONSchema;
use portablenote_core::domain::types::{Block, Document, Edge, Manifest, Section, Subsection};
use serde_json::Value;
use uuid::Uuid;

fn spec_schemas_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("..").join("spec").join("schemas")
}

fn load_schema_value(name: &str) -> Value {
    let path = spec_schemas_dir().join(format!("{name}.schema.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read schema {}: {e}", path.display()));
    let mut value: Value =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("failed to parse schema {}: {e}", name));
    // Strip relative $id so the compiler does not need a base URL (avoids "relative URL without a base").
    if let Some(obj) = value.as_object_mut() {
        obj.remove("$id");
    }
    value
}

fn compile_schema(schema: &Value) -> JSONSchema {
    JSONSchema::compile(schema).unwrap_or_else(|e| panic!("failed to compile schema: {e}"))
}

fn load_and_compile_schema(name: &str) -> JSONSchema {
    let schema = load_schema_value(name);
    compile_schema(&schema)
}

/// Load block-graph.schema.json and compile only the edge definition for validating a single Edge.
fn load_and_compile_edge_schema() -> JSONSchema {
    let full = load_schema_value("block-graph");
    let edge_def = full
        .get("$defs")
        .and_then(|d| d.get("edge"))
        .unwrap_or_else(|| panic!("block-graph.schema.json missing $defs.edge"));
    compile_schema(edge_def)
}

fn assert_valid(validator: &JSONSchema, instance: &Value) {
    let result = validator.validate(instance);
    if let Err(errors) = result {
        let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!("instance failed schema validation: {}", messages.join("; "));
    }
}

#[test]
fn block_serialization_validates_against_schema() {
    let schema = load_and_compile_schema("block");
    let block = Block {
        id: Uuid::nil(),
        name: "Test Block".to_string(),
        content: "Hello world.\n".to_string(),
        created: Utc::now(),
        modified: Utc::now(),
    };
    let value = serde_json::to_value(&block).expect("Block serialization");
    assert_valid(&schema, &value);
}

#[test]
fn edge_serialization_validates_against_schema() {
    let schema = load_and_compile_edge_schema();
    let edge = Edge {
        id: Uuid::nil(),
        source: Uuid::nil(),
        target: Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap(),
    };
    let value = serde_json::to_value(&edge).expect("Edge serialization");
    assert_valid(&schema, &value);
}

#[test]
fn document_serialization_validates_against_schema() {
    let schema = load_and_compile_schema("document");
    let root_id = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
    let section_block = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
    let subsection_block = Uuid::parse_str("00000000-0000-4000-a000-000000000003").unwrap();
    let doc = Document {
        id: Uuid::nil(),
        root: root_id,
        sections: vec![Section {
            block: section_block,
            subsections: vec![Subsection {
                block: subsection_block,
            }],
        }],
    };
    let value = serde_json::to_value(&doc).expect("Document serialization");
    assert_valid(&schema, &value);
}

#[test]
fn manifest_serialization_validates_against_schema() {
    let schema = load_and_compile_schema("manifest");
    let manifest = Manifest {
        vault_id: Uuid::nil(),
        spec_version: "0.1.0".to_string(),
        format: "markdown".to_string(),
        checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_string(),
        previous_checksum: None,
    };
    let value = serde_json::to_value(&manifest).expect("Manifest serialization");
    assert_valid(&schema, &value);
}
