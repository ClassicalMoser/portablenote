//! One module per spec command. Each exposes a single `execute` function that
//! accepts port trait references and returns either a domain event (single-store)
//! or a result struct (multi-store) for the adapter to apply.

pub mod add_block;
pub mod add_document;
pub mod add_edge;
pub mod init_vault;
pub mod append_section;
pub mod append_subsection;
pub mod delete_block_cascade;
pub mod delete_block_safe;
pub mod delete_document;
pub mod mutate_block_content;
pub mod remove_edge;
pub mod remove_section;
pub mod rename_block;
pub mod reorder_sections;
