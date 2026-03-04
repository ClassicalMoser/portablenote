# portablenote-core Architecture

This document describes the architecture of the **reference Rust implementation** of the PortableNote spec. It is not normative — other implementations may make entirely different architectural choices as long as they pass the compliance suite in `spec/compliance/`.

---

## Hexagonal Layers

The crate is split into two top-level modules that map to the inner rings of a hexagonal architecture:

```
core/src/
  domain/        Pure types and functions. No I/O, no trait objects, no ports.
  application/   Port traits, use cases, result types. Orchestrates domain + ports.
```

Adapters (persistence, format parsing, rendering, search) live *outside* this crate, in separate crates or binaries that depend on `portablenote-core`. The core never imports adapter code.

### Control flow

```
Client (UI / CLI / test harness)
  → Use case function  (application/)
    → Pure domain functions  (domain/)
    ← Domain-typed result
  ← Per-command result struct  (application/results)
  → Infrastructure adapter persists result atomically
```

The use case reads through port traits, calls pure domain functions, and returns a result struct. It never writes through ports for multi-store operations — that responsibility belongs to the adapter (see Changeset Pattern below).

---

## Domain Layer (`domain/`)

Everything here is pure. Functions take owned or borrowed domain types and return domain types or `Result<_, DomainError>`. No `dyn`, no trait objects, no I/O.

| Module | Responsibility |
|---|---|
| `types` | Entity structs: `Block`, `Edge`, `Document`, `Manifest`, `BlockGraph`, `Vault`. `Vault` is a read-only snapshot for validation/import, not the unit of command execution. |
| `blocks` | Pure constructors and transforms: `create`, `apply_rename`, `apply_content`, `propagate_rename`, `revert_refs`. |
| `content` | Markdown content helpers: heading detection, inline ref extraction, footer annotation parsing, rename/revert propagation. |
| `documents` | Document projection (`project`) — walks a `Document` definition and collects the referenced blocks in render order. |
| `edges` | Edge construction. |
| `commands` | Command data structs. Carry payload only — no behavior. |
| `events` | Event data structs emitted by successful commands. |
| `error` | `DomainError` (command rejection) and `Violation` / `ViolationDetails` (invariant failures at validation time). |
| `invariants` | Full-vault validation. Checks all spec invariants against a `Vault` snapshot. Used at open/import time, not during normal command execution. |
| `checksum` | SHA-256 over canonical serialization of all source artifacts. Includes `is_drifted` for load-time drift detection. |
| `queries` | Pure read-only functions over a `Vault` snapshot: `edges_for`, `orphans`, `backlinks`, `resolve_name`, `list_blocks`. |

---

## Application Layer (`application/`)

### Port Traits (`ports`)

Interfaces that infrastructure adapters implement. Defined here so use cases can depend on abstractions without knowing the concrete storage backend.

| Trait | Purpose |
|---|---|
| `BlockStore` | Read/write access to the block heap. `get`, `list`, `find_by_ref`, `save`, `delete`. |
| `GraphStore` | Read/write access to the reference graph. `get_edge`, `incoming`, `edges_for`, `save_edge`, `remove_edges`. |
| `DocumentStore` | Read/write access to document definitions. `get`, `save`, `delete`. |
| `NameIndex` | Human-readable name → UUID resolution. `resolve`, `set`, `remove`. |

All traits are annotated with `#[cfg_attr(test, mockall::automock)]` so that `MockBlockStore`, `MockGraphStore`, etc. are generated automatically for test builds.

### Result Types (`results`)

Per-command result structs returned by multi-store use cases. Each struct carries *all* entities that need to be persisted — the adapter reads the struct and writes atomically.

- `AddBlockResult` — the new `Block` and a `BlockAdded` event.
- `RenameBlockResult` — the renamed `Block`, all propagated `Block`s (with updated inline refs), the old name to remove, and a `BlockRenamed` event.
- `DeleteBlockSafeResult` — the block ID, reverted blocks, outgoing edge IDs to remove, name to remove, and a `BlockDeleted` event.
- `DeleteBlockCascadeResult` — same shape but includes all edge IDs (incoming + outgoing).

### Use Cases (`use_cases/`)

One module per command. Each exposes an `execute` function.

Use cases fall into two categories based on how many stores they touch:

**Multi-store (read-only ports, return result struct):**
- `add_block` — reads `BlockStore` + `NameIndex`, returns `AddBlockResult`
- `rename_block` — reads `BlockStore` + `NameIndex`, returns `RenameBlockResult`
- `delete_block_safe` — reads `BlockStore` + `GraphStore`, returns `DeleteBlockSafeResult`
- `delete_block_cascade` — reads `BlockStore` + `GraphStore`, returns `DeleteBlockCascadeResult`

**Single-store (mutable port, writes directly):**
- `mutate_block_content`, `add_document`, `append_section`, `append_subsection`, `remove_section`, `reorder_sections`, `delete_document`, `add_edge`, `remove_edge`

Single-store use cases write through a `&mut dyn Store` directly because atomicity is trivial when only one store is involved.

---

## The Changeset Pattern

The central architectural decision for correctness: **multi-store use cases never write**. They take `&dyn` (read-only borrows of) port traits, compute the full set of changes, and return a result struct describing everything the adapter must persist.

Why this matters: `rename_block` touches `BlockStore` (the renamed block + all blocks with updated inline refs) and `NameIndex` (remove old name, register new). If the use case wrote to both stores directly and one write failed, the vault would be left in an inconsistent state. By returning a `RenameBlockResult`, the use case pushes atomicity to the infrastructure boundary where the adapter can use a transaction, a write-ahead log, or whatever mechanism is appropriate for the storage backend.

The use case is pure in the sense that matters: given the same port reads, it always returns the same result. Side effects are concentrated at the adapter boundary.

---

## Save Model

The spec defines commands and events but does not prescribe a save model. This implementation uses:

- **Manual save** as the primary model. In-progress edits are local to the client until explicitly committed.
- **Autosave on close** and **periodic autosave** (configurable interval, default 5 minutes) as supplements.
- **Optimistic concurrency** via `base_version` — a monotonically increasing mutation counter on the `Vault` snapshot. Every command struct carries a `base_version` field. The application layer (above the use cases) checks whether the relevant artifact changed since `base_version` before dispatching. If it has, the client is notified and decides how to proceed. This is deliberately not enforced by use cases — the domain layer is single-writer.

Name propagations from renames that occurred since `base_version` are corrected silently. Deleted referenced blocks are handled by cascade remediation (inline refs reverted to plain text). Only content or document definition conflicts surface to the user.

---

## Testing Strategy

### Colocated unit tests

Every domain module and use case module contains a `#[cfg(test)] mod tests` block with focused unit tests. Domain function tests call the functions directly. Use case tests mock port traits via `mockall`:

```rust
let mut blocks = MockBlockStore::new();
blocks.expect_get()
    .with(eq(id()))
    .return_once(move |_| Some(make_block(id(), "Alpha")));

let result = execute(&blocks, &names, id(), "Beta").unwrap();
assert_eq!(result.renamed.name, "Beta");
```

Mocks are unit-level — they test that the use case issues the right reads and produces the right result. They do not simulate realistic multi-store coordination.

### Integration tests (`tests/`)

Integration tests load the compliance fixtures from `spec/compliance/` and run domain validation:

- `fixture_loader_test` — loads every valid and invalid vault fixture, asserts pass/fail.
- `invariants_test` — validates specific invariant violations against invalid fixtures.
- `queries_test` — runs query functions against loaded vault snapshots.

Mutation scenarios in `spec/compliance/mutations/` are a known gap — not yet wired into automated integration tests.

---

## Client Separation

The core is a library crate. It has no knowledge of transport, UI framework, or platform. Clients are separate projects that depend on the core:

- **Desktop** — A Tauri shell that runs the core in-process and bridges commands/events via IPC. The frontend is a SolidJS app.
- **Web** — A server binary that exposes the core over HTTP. The same SolidJS frontend talks to it via fetch.
- **CLI / other** — Any binary that depends on `portablenote-core` and wires up adapters.

Tauri is one way to build a desktop client. It is not required by the spec and not the only option.
