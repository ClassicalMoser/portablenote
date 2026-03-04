#![allow(dead_code)] // shared test infra — not every binary uses every function

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use portablenote_core::domain::types::*;
use uuid::Uuid;

/// Resolve the spec/compliance/ directory relative to the crate root.
pub fn spec_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("..").join("spec").join("compliance")
}

/// Load a vault from a fixture directory path.
/// The path should point to a directory containing `portablenote/`.
pub fn load_vault(vault_dir: &Path) -> Vault {
    let pn_dir = vault_dir.join("portablenote");

    let manifest = load_manifest(&pn_dir.join("manifest.json"));
    let graph = load_block_graph(&pn_dir.join("block-graph.json"));
    let blocks = load_blocks(&pn_dir.join("blocks"));
    let documents = load_documents(&pn_dir.join("documents"));
    let names = load_names(&pn_dir.join("names.json"));

    Vault {
        manifest,
        blocks,
        graph,
        documents,
        names,
        version: 0,
    }
}

fn load_manifest(path: &Path) -> Manifest {
    let content = fs::read_to_string(path).expect("Failed to read manifest.json");
    serde_json::from_str(&content).expect("Failed to parse manifest.json")
}

fn load_names(path: &Path) -> HashMap<String, Uuid> {
    let content = fs::read_to_string(path).expect("Failed to read names.json");
    serde_json::from_str(&content).expect("Failed to parse names.json")
}

fn load_block_graph(path: &Path) -> BlockGraph {
    let content = fs::read_to_string(path).expect("Failed to read block-graph.json");
    serde_json::from_str(&content).expect("Failed to parse block-graph.json")
}

fn load_blocks(blocks_dir: &Path) -> HashMap<Uuid, Block> {
    let mut blocks = HashMap::new();

    if !blocks_dir.exists() {
        return blocks;
    }

    for entry in fs::read_dir(blocks_dir).expect("Failed to read blocks directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let raw = fs::read_to_string(&path).expect("Failed to read block file");
        let block = parse_block_file(&raw);
        blocks.insert(block.id, block);
    }

    blocks
}

/// Scan a blocks directory for duplicate UUIDs in metadata.
/// Returns a list of UUIDs that appear in more than one file.
/// This detects corruption that HashMap loading would silently mask.
pub fn find_duplicate_uuids(vault_dir: &Path) -> Vec<Uuid> {
    let blocks_dir = vault_dir.join("portablenote").join("blocks");
    if !blocks_dir.exists() {
        return Vec::new();
    }

    let mut seen: HashMap<Uuid, usize> = HashMap::new();

    for entry in fs::read_dir(&blocks_dir).expect("Failed to read blocks directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let raw = fs::read_to_string(&path).expect("Failed to read block file");
        let (metadata_str, _) = extract_html_comment_metadata(&raw);
        let metadata = parse_yaml_metadata(&metadata_str);

        if let Some(id) = metadata.get("id").and_then(|s| Uuid::parse_str(s).ok()) {
            *seen.entry(id).or_insert(0) += 1;
        }
    }

    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id)
        .collect()
}

fn load_documents(docs_dir: &Path) -> HashMap<Uuid, Document> {
    let mut documents = HashMap::new();

    if !docs_dir.exists() {
        return documents;
    }

    for entry in fs::read_dir(docs_dir).expect("Failed to read documents directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).expect("Failed to read document file");
        let doc: Document =
            serde_json::from_str(&content).expect("Failed to parse document file");
        documents.insert(doc.id, doc);
    }

    documents
}

/// Parse a block `.md` file with HTML-comment metadata.
///
/// Expected format:
/// ```text
/// <!--
/// id: <uuid>
/// name: <string>
/// created: <iso8601>
/// modified: <iso8601>
/// -->
///
/// <content>
/// ```
fn parse_block_file(raw: &str) -> Block {
    let (metadata_str, content) = extract_html_comment_metadata(raw);

    let metadata: HashMap<String, String> = parse_yaml_metadata(&metadata_str);

    let id = metadata
        .get("id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("Block metadata must contain a valid 'id' field");

    let name = metadata
        .get("name")
        .cloned()
        .unwrap_or_default();

    let created = metadata
        .get("created")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_default();

    let modified = metadata
        .get("modified")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_default();

    Block {
        id,
        name,
        content,
        created,
        modified,
    }
}

/// Extract the YAML content from `<!-- ... -->` and the remaining body content.
fn extract_html_comment_metadata(raw: &str) -> (String, String) {
    let trimmed = raw.trim_start();

    if let Some(after_open) = trimmed.strip_prefix("<!--") {
        if let Some(close_pos) = after_open.find("-->") {
            let yaml = after_open[..close_pos].trim().to_string();
            let rest = after_open[close_pos + 3..].to_string();
            let content = rest.trim_start_matches('\n').trim_start_matches('\r');
            return (yaml, content.to_string());
        }
    }

    (String::new(), raw.to_string())
}

fn parse_yaml_metadata(yaml_str: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in yaml_str.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}

/// Load the `_expected_error.json` for an invalid vault fixture.
pub fn load_expected_error(vault_dir: &Path) -> serde_json::Value {
    let path = vault_dir.join("_expected_error.json");
    let content = fs::read_to_string(&path).expect("Failed to read _expected_error.json");
    serde_json::from_str(&content).expect("Failed to parse _expected_error.json")
}
