//! Data-driven compliance tests: one `#[test]` per mutation scenario JSON.
//!
//! Each test loads the scenario file, hydrates in-memory stores from the
//! fixture vault, dispatches the command, and evaluates every assertion.
//! Adding a new JSON scenario to `spec/compliance/mutations/` and a
//! one-line macro invocation here is all that's needed for coverage.

mod common;

mod support;

macro_rules! compliance {
    ($name:ident, $file:expr) => {
        #[test]
        fn $name() {
            support::harness::run_scenario($file);
        }
    };
}

// Block commands
compliance!(add_block, "add-block.json");
compliance!(add_block_duplicate_name, "add-block-duplicate-name.json");
compliance!(rename_block, "rename-block.json");
compliance!(mutate_block_content, "mutate-block-content.json");
compliance!(mutate_block_content_heading_rejected, "mutate-block-content-heading-rejected.json");
compliance!(delete_block_safe, "delete-block-safe.json");
compliance!(delete_block_safe_orphan, "delete-block-safe-orphan.json");
compliance!(delete_block_cascade, "delete-block-cascade.json");

// Edge commands
compliance!(add_edge, "add-edge.json");
compliance!(add_edge_dangling_target, "add-edge-dangling-target.json");
compliance!(remove_edge, "remove-edge.json");
compliance!(remove_edge_not_found, "remove-edge-not-found.json");

// Document commands
compliance!(add_document, "add-document.json");
compliance!(add_document_missing_root, "add-document-missing-root.json");
compliance!(delete_document, "delete-document.json");
compliance!(append_section, "append-section.json");
compliance!(append_subsection, "append-subsection.json");
compliance!(remove_section, "remove-section.json");
compliance!(reorder_sections, "reorder-sections.json");
