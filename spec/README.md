# PortableNote Specification

**Version:** 0.1.0-draft  
**License:** Apache 2.0

This directory contains the PortableNote format specification and its machine-readable compliance artifacts. These are intended to be consumed by any conforming implementation as a dependency or submodule.

The specification is **implementation-agnostic**. It defines the vault format, artifact schemas, mutation commands, validation invariants, and behavioral contracts. It does not prescribe architecture, language, or tooling.

## Normative Reference

The full specification is in [`portablenote-spec.md`](portablenote-spec.md).

## Contents

```
spec/
  portablenote-spec.md        Normative specification document
  schemas/                    JSON schemas for vault artifacts
    block.schema.json         Block type (canonical; referenced by vault-writes and journal)
    manifest.schema.json      Vault manifest (identity, checksum chain)
    block-graph.schema.json   Block reference graph (edges)
    document.schema.json      Document composition definition
    names.schema.json         Name-to-UUID index
    vault-writes.schema.json  Vault write algebra (8 primitive mutation kinds + before-image)
    journal.schema.json       Commit journal (ephemeral; crash-recovery write-ahead log)
  compliance/                 Test fixtures for conformance testing
    valid/                    Vault directories that must pass validation
    invalid/                  Vault directories that must fail with specific errors
    mutations/                Scenario files: initial vault + command + expected outcome
```

## Compliance

A conforming implementation must:

1. **Accept** every vault in `compliance/valid/` without errors. Vaults in `valid/` may include drifted fixtures (e.g. `minimal-drifted`) where `manifest.checksum` is wrong but content satisfies all invariants; the implementation must allow mutation after the gate (checksum mismatch → revalidate → pass).
2. **Reject** every vault in `compliance/invalid/` with the error described in its `_expected_error.json`. Invalid vaults fail full validation (§6); the implementation must not permit any mutation until remediation (§5 Mutation gate).
3. **Produce the expected outcome** for every scenario in `compliance/mutations/`. Scenarios that use a drifted vault (e.g. `add-block-drifted-vault.json`) assert that the mutation gate permits the command when revalidation passes.

Implementations run the compliance suite as part of CI. The spec repo does not contain engine or UI code — only documentation, schemas, and tests.

### Mutation Scenario Format

Each `.json` file in `compliance/mutations/` has the following shape:

```json
{
  "description": "Human-readable description of the scenario",
  "initial_vault": "relative path to a vault directory in compliance/valid/ or compliance/invalid/",
  "command": { "type": "AddBlock", "payload": { ... } },
  "expected": {
    "result": "success" | "rejected",
    "error": "optional error description if rejected",
    "assertions": [ ... ]
  }
}
```

Assertions describe the expected state after the command: block exists/doesn't exist, edge present/absent, name index updated, inline refs updated, etc.

### Keeping the Suite in Sync

Every spec change should yield updated fixtures and scenarios. The tests need a well-defined interface (e.g. "load vault from path", "execute command with payload", "assert vault state or error") so that any implementation — regardless of language — can run the same scenarios. Language-agnostic scenarios (JSON describing initial vault + command + expected outcome) keep the suite portable.
