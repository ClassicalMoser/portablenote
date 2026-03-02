# PortableNote Specification

**Version:** 0.1.0-draft  
**License:** Apache 2.0

This directory contains the machine-readable specification artifacts for the PortableNote format. These are intended to be consumed by any conforming implementation as a dependency or submodule.

## Contents

```
spec/
  schemas/                  JSON schemas for vault artifacts
    manifest.schema.json    Vault manifest
    block-graph.schema.json Block reference graph
    document.schema.json    Document composition definition
  compliance/               Test fixtures for conformance testing
    valid/                  Vault directories that must pass validation
    invalid/                Vault directories that must fail with specific errors
    mutations/              Scenario files: initial vault + command + expected outcome
```

## Usage

Implementations validate their persistence layer against the JSON schemas and run the compliance suite as part of CI. A conforming implementation must:

1. Accept every vault in `compliance/valid/` without errors.
2. Reject every vault in `compliance/invalid/` with the error described in its `_expected_error.json`.
3. Produce the expected outcome for every scenario in `compliance/mutations/`.

## Mutation Scenario Format

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

## Normative Reference

The full specification is in [`portablenote-spec.md`](../portablenote-spec.md) at the repository root.
