mod common;

use portablenote_core::domain::error::ViolationDetails;
use portablenote_core::domain::invariants::validate_vault;

// --- Valid vaults: zero violations ---

#[test]
fn valid_minimal_passes_all_invariants() {
    let dir = common::spec_dir().join("valid").join("minimal");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);
    assert!(
        violations.is_empty(),
        "minimal vault should have no violations, got: {violations:?}"
    );
}

#[test]
fn valid_with_refs_passes_all_invariants() {
    let dir = common::spec_dir().join("valid").join("with-refs");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);
    assert!(
        violations.is_empty(),
        "with-refs vault should have no violations, got: {violations:?}"
    );
}

#[test]
fn valid_with_documents_passes_all_invariants() {
    let dir = common::spec_dir().join("valid").join("with-documents");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);
    assert!(
        violations.is_empty(),
        "with-documents vault should have no violations, got: {violations:?}"
    );
}

#[test]
fn valid_with_orphans_passes_all_invariants() {
    let dir = common::spec_dir().join("valid").join("with-orphans");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);
    assert!(
        violations.is_empty(),
        "with-orphans vault should have no violations, got: {violations:?}"
    );
}

// --- Invalid vaults: specific violations ---

#[test]
fn invalid_dangling_uuid_detected() {
    let dir = common::spec_dir().join("invalid").join("dangling-uuid");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);

    assert!(!violations.is_empty(), "should detect dangling UUID");
    let v = violations
        .iter()
        .find(|v| matches!(v.details, ViolationDetails::DanglingEdgeUuid { .. }))
        .expect("should have a DanglingEdgeUuid violation");
    assert!(
        v.description.contains("exist"),
        "description should mention existence: {}",
        v.description
    );
}

#[test]
fn invalid_duplicate_uuid_detected() {
    let dir = common::spec_dir().join("invalid").join("duplicate-uuid");
    let duplicates = common::find_duplicate_uuids(&dir);

    assert!(
        !duplicates.is_empty(),
        "should detect duplicate UUIDs across block files"
    );
    assert!(
        duplicates.contains(
            &uuid::Uuid::parse_str("51000000-0000-4000-a000-000000000001").unwrap()
        ),
        "expected duplicated UUID not found in: {duplicates:?}"
    );
}

#[test]
fn invalid_duplicate_name_detected() {
    let dir = common::spec_dir().join("invalid").join("duplicate-name");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);

    assert!(
        !violations.is_empty(),
        "should detect duplicate name (case-insensitive)"
    );
    let v = violations
        .iter()
        .find(|v| matches!(v.details, ViolationDetails::DuplicateName { .. }))
        .expect("should have a DuplicateName violation");
    assert!(
        v.description.contains("case-insensitive"),
        "description should mention case-insensitive: {}",
        v.description
    );
}

#[test]
fn invalid_heading_in_block_detected() {
    let dir = common::spec_dir().join("invalid").join("heading-in-block");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);

    assert!(!violations.is_empty(), "should detect heading in block");
    let v = violations
        .iter()
        .find(|v| matches!(v.details, ViolationDetails::HeadingInContent { .. }))
        .expect("should have a HeadingInContent violation");
    assert!(
        v.description.contains("heading"),
        "description should mention heading: {}",
        v.description
    );
}

#[test]
fn invalid_reserved_in_name_detected() {
    let dir = common::spec_dir().join("invalid").join("reserved-in-name");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);

    assert!(!violations.is_empty(), "should detect reserved character in name");
    let v = violations
        .iter()
        .find(|v| matches!(v.details, ViolationDetails::NameContainsReservedCharacters { .. }))
        .expect("should have a NameContainsReservedCharacters violation");
    assert!(
        v.description.contains("reserved"),
        "description should mention reserved: {}",
        v.description
    );
}

#[test]
fn invalid_missing_metadata_detected() {
    let dir = common::spec_dir().join("invalid").join("missing-frontmatter");
    let vault = common::load_vault(&dir);
    let violations = validate_vault(&vault);

    assert!(
        !violations.is_empty(),
        "should detect missing metadata field"
    );
    let v = violations
        .iter()
        .find(|v| matches!(v.details, ViolationDetails::MissingMetadataField { .. }))
        .expect("should have a MissingMetadataField violation");
    assert!(
        v.description.contains("name"),
        "description should mention missing name: {}",
        v.description
    );
}
