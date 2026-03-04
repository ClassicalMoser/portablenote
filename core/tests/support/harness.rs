//! Mutation compliance harness.
//!
//! Loads a scenario JSON, hydrates in-memory stores from the fixture vault,
//! dispatches the command to the appropriate use case, applies changesets for
//! multi-store results, and evaluates every post-mutation assertion.

#![allow(dead_code)] // shared test infra — not every binary uses every function

use std::fs;
use std::path::Path;

use serde_json::json;
use uuid::Uuid;

use portablenote_core::application::use_cases::{
    add_block, add_document, add_edge, append_section, append_subsection, delete_block_cascade,
    delete_block_safe, delete_document, mutate_block_content, remove_edge, remove_section,
    rename_block, reorder_sections,
};
use super::assertion;
use super::changeset;
use super::factory;
use super::in_memory::VaultStores;
use super::scenario::{MutationScenario, ScenarioCommand};

/// Lightweight representation of a use case outcome, decoupled from
/// concrete event structs so the assertion engine stays generic.
pub enum CommandOutcome {
    Success {
        event_name: String,
        event_fields: serde_json::Map<String, serde_json::Value>,
    },
    Rejected {
        error: String,
    },
}

/// Run a single mutation compliance scenario from a JSON file path
/// relative to the `spec/compliance/mutations/` directory.
pub fn run_scenario(filename: &str) {
    let mutations_dir = mutations_dir();
    let json_path = mutations_dir.join(filename);
    let raw = fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("failed to read scenario {filename}: {e}"));
    let scenario: MutationScenario =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {filename}: {e}"));

    let vault_path = mutations_dir
        .join(&scenario.initial_vault)
        .canonicalize()
        .unwrap_or_else(|e| {
            panic!(
                "failed to resolve initial_vault '{}': {e}",
                scenario.initial_vault
            )
        });

    let vault = crate::common::load_vault(&vault_path);
    let mut stores = factory::from_vault(&vault);

    let outcome = dispatch(&scenario.command, &mut stores);

    if scenario.expected.result == "rejected" {
        let CommandOutcome::Rejected { error } = &outcome else {
            panic!(
                "[{}] expected rejection but command succeeded",
                scenario.description
            );
        };
        if let Some(expected_error) = &scenario.expected.error {
            assert!(
                error.contains(expected_error.as_str()),
                "[{}] error message '{}' does not contain expected '{}'",
                scenario.description,
                error,
                expected_error
            );
        }
        return;
    }

    assert_eq!(
        scenario.expected.result, "success",
        "[{}] unexpected result value: {}",
        scenario.description, scenario.expected.result
    );

    if let CommandOutcome::Rejected { error } = &outcome {
        panic!(
            "[{}] expected success but got rejection: {error}",
            scenario.description
        );
    }

    for a in &scenario.expected.assertions {
        assertion::evaluate(a, &stores, &outcome);
    }
}

// ---------------------------------------------------------------------------
// Command dispatcher
// ---------------------------------------------------------------------------

fn dispatch(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    match cmd.kind.as_str() {
        "AddBlock" => dispatch_add_block(cmd, stores),
        "RenameBlock" => dispatch_rename_block(cmd, stores),
        "DeleteBlock" => dispatch_delete_block(cmd, stores),
        "MutateBlockContent" => dispatch_mutate_block_content(cmd, stores),
        "AddEdge" => dispatch_add_edge(cmd, stores),
        "RemoveEdge" => dispatch_remove_edge(cmd, stores),
        "AddDocument" => dispatch_add_document(cmd, stores),
        "DeleteDocument" => dispatch_delete_document(cmd, stores),
        "AppendSection" => dispatch_append_section(cmd, stores),
        "AppendSubsection" => dispatch_append_subsection(cmd, stores),
        "RemoveSection" => dispatch_remove_section(cmd, stores),
        "ReorderSections" => dispatch_reorder_sections(cmd, stores),
        other => panic!("unknown command type: {other}"),
    }
}

fn payload_uuid(cmd: &ScenarioCommand, key: &str) -> Uuid {
    let s = cmd.payload[key]
        .as_str()
        .unwrap_or_else(|| panic!("payload missing string field '{key}'"));
    Uuid::parse_str(s).unwrap_or_else(|_| panic!("payload field '{key}' is not a valid UUID"))
}

fn payload_str(cmd: &ScenarioCommand, key: &str) -> String {
    cmd.payload[key]
        .as_str()
        .unwrap_or_else(|| panic!("payload missing string field '{key}'"))
        .to_string()
}

// ---------------------------------------------------------------------------
// Per-command dispatch functions
// ---------------------------------------------------------------------------

fn dispatch_add_block(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let id = payload_uuid(cmd, "id");
    let name = payload_str(cmd, "name");
    let content = payload_str(cmd, "content");

    match add_block::execute(&stores.blocks, &stores.names, id, &name, &content) {
        Ok(result) => {
            let event_fields = json_map(json!({
                "block_id": result.event.block_id.to_string(),
                "name": result.event.name,
            }));
            changeset::apply_add_block(stores, result);
            CommandOutcome::Success {
                event_name: "BlockAdded".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_rename_block(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let block_id = payload_uuid(cmd, "block_id");
    let new_name = payload_str(cmd, "new_name");

    match rename_block::execute(&stores.blocks, &stores.names, block_id, &new_name) {
        Ok(result) => {
            let event_fields = json_map(json!({
                "block_id": result.event.block_id.to_string(),
                "old_name": result.event.old_name,
                "new_name": result.event.new_name,
            }));
            changeset::apply_rename_block(stores, result);
            CommandOutcome::Success {
                event_name: "BlockRenamed".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_delete_block(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let block_id = payload_uuid(cmd, "block_id");
    let mode = payload_str(cmd, "mode");

    match mode.as_str() {
        "safe" => match delete_block_safe::execute(&stores.blocks, &stores.graph, block_id) {
            Ok(result) => {
                let event_fields = json_map(json!({
                    "block_id": result.event.block_id.to_string(),
                }));
                changeset::apply_delete_block_safe(stores, result);
                CommandOutcome::Success {
                    event_name: "BlockDeleted".to_string(),
                    event_fields,
                }
            }
            Err(e) => CommandOutcome::Rejected {
                error: e.to_string(),
            },
        },
        "cascade" => {
            match delete_block_cascade::execute(&stores.blocks, &stores.graph, block_id) {
                Ok(result) => {
                    let event_fields = json_map(json!({
                        "block_id": result.event.block_id.to_string(),
                    }));
                    changeset::apply_delete_block_cascade(stores, result);
                    CommandOutcome::Success {
                        event_name: "BlockDeleted".to_string(),
                        event_fields,
                    }
                }
                Err(e) => CommandOutcome::Rejected {
                    error: e.to_string(),
                },
            }
        }
        other => panic!("unknown DeleteBlock mode: {other}"),
    }
}

fn dispatch_mutate_block_content(
    cmd: &ScenarioCommand,
    stores: &mut VaultStores,
) -> CommandOutcome {
    let block_id = payload_uuid(cmd, "block_id");
    let content = payload_str(cmd, "content");

    match mutate_block_content::execute(&mut stores.blocks, block_id, &content) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "block_id": event.block_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "BlockContentMutated".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_add_edge(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let id = payload_uuid(cmd, "id");
    let source = payload_uuid(cmd, "source");
    let target = payload_uuid(cmd, "target");

    match add_edge::execute(&stores.blocks, &mut stores.graph, id, source, target) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "edge_id": event.edge_id.to_string(),
                "source": event.source.to_string(),
                "target": event.target.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "EdgeAdded".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_remove_edge(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let edge_id = payload_uuid(cmd, "edge_id");

    match remove_edge::execute(&mut stores.graph, edge_id) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "edge_id": event.edge_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "EdgeRemoved".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_add_document(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let id = payload_uuid(cmd, "id");
    let root = payload_uuid(cmd, "root");

    match add_document::execute(&stores.blocks, &mut stores.documents, id, root) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "document_id": event.document_id.to_string(),
                "root_block_id": event.root_block_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "DocumentAdded".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_delete_document(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let document_id = payload_uuid(cmd, "document_id");

    match delete_document::execute(&mut stores.documents, document_id) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "document_id": event.document_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "DocumentDeleted".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_append_section(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let document_id = payload_uuid(cmd, "document_id");
    let block_id = payload_uuid(cmd, "block_id");

    match append_section::execute(&stores.blocks, &mut stores.documents, document_id, block_id) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "document_id": event.document_id.to_string(),
                "block_id": event.block_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "SectionAppended".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_append_subsection(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let document_id = payload_uuid(cmd, "document_id");
    let section_block_id = payload_uuid(cmd, "section_block_id");
    let block_id = payload_uuid(cmd, "block_id");

    match append_subsection::execute(
        &stores.blocks,
        &mut stores.documents,
        document_id,
        section_block_id,
        block_id,
    ) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "document_id": event.document_id.to_string(),
                "block_id": event.block_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "SectionAppended".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_remove_section(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let document_id = payload_uuid(cmd, "document_id");
    let block_id = payload_uuid(cmd, "block_id");

    match remove_section::execute(&mut stores.documents, document_id, block_id) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "document_id": event.document_id.to_string(),
                "block_id": event.block_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "SectionRemoved".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

fn dispatch_reorder_sections(cmd: &ScenarioCommand, stores: &mut VaultStores) -> CommandOutcome {
    let document_id = payload_uuid(cmd, "document_id");
    let order: Vec<Uuid> = cmd.payload["section_order"]
        .as_array()
        .expect("section_order must be an array")
        .iter()
        .map(|v| {
            Uuid::parse_str(v.as_str().expect("section_order element must be a string"))
                .expect("section_order element must be a valid UUID")
        })
        .collect();

    match reorder_sections::execute(&mut stores.documents, document_id, order) {
        Ok(event) => {
            let event_fields = json_map(json!({
                "document_id": event.document_id.to_string(),
            }));
            CommandOutcome::Success {
                event_name: "SectionsReordered".to_string(),
                event_fields,
            }
        }
        Err(e) => CommandOutcome::Rejected {
            error: e.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mutations_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("..")
        .join("spec")
        .join("compliance")
        .join("mutations")
}

/// Extract the inner map from a `serde_json::Value::Object`.
fn json_map(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    match value {
        serde_json::Value::Object(m) => m,
        _ => unreachable!("json! macro with {{}} always produces Object"),
    }
}
