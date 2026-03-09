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

Adapters (persistence, rendering, search) live *outside* this crate, in separate crates or binaries that depend on `portablenote-core`. The core never imports adapter code. Markdown is the native format: block file parsing and serialization (`.md` ↔ `Block`) live in the domain layer (`domain/format`, `domain/content`).

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

Everything here is pure. Functions take owned or borrowed domain types and return domain types or `Result<_, DomainError>`. No `dyn`, no trait objects, no I/O. This layer implements the **spec’s domain-level definitions**: vault state (blocks, graph, documents, manifest checksum chain), invariants (§6), checksum computation, and the mutation gate rule (allow vs StaleState vs RemediationRequired). The spec’s commit model (base, pending diff, rebase, overlap) is defined at domain level in the spec; rebase/overlap logic is not yet implemented in this crate.

| Module | Responsibility |
|---|---|
| `types` | Entity structs: `Block`, `Edge`, `Document`, `Manifest`, `BlockGraph`, `Vault`. `Vault` is a read-only snapshot for validation/import, not the unit of command execution. |
| `blocks` | Pure constructors and transforms: `create`, `apply_rename`, `apply_content`, `propagate_rename`, `revert_refs`. |
| `content` | Markdown content helpers: heading detection. (Block-reference link parsing lives in application `block_file`.) |
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
| `BlockStore` | Read/write access to the block heap. `get`, `list`, `find_by_target`, `save`, `delete`. |
| `GraphStore` | Read/write access to the reference graph. `get_edge`, `incoming`, `edges_for`, `save_edge`, `remove_edges`. |
| `DocumentStore` | Read/write access to document definitions. `get`, `save`, `delete`. |
| `NameIndex` | Human-readable name → UUID resolution. `resolve`, `set`, `remove`. |
| `ManifestStore` | Read/write the vault manifest (checksum chain). `get`, `write`. The **save snapshot** boundary for reconstructible atomic commits. |
| `MutationGate` | §5 mutation gate: `allow_mutation(expected_checksum)` — adapter builds a full vault from its state; optional OCC (expected_checksum must match manifest.checksum); then checksum drift check and (on mismatch) full validation; returns `Ok(())`, `StaleState`, or `RemediationRequired`. Keeps gate logic in the core (`gate::mutation_gate`) and avoids adding "list everything" to other ports. |
| `Clock` | Source of current time (`now()`). Injected so domain and use cases stay testable; infra provides `SystemClock`. |

All traits are annotated with `#[cfg_attr(test, mockall::automock)]` so that `MockBlockStore`, `MockGraphStore`, etc. are generated automatically for test builds.

### Commit protocol (§5a)

The spec’s reconstructible atomic commit model is implemented by the **adapter**, not the use cases. Use cases return `CommandResult` (writes); the adapter is responsible for:

1. **Journal** — Write `.journal` (expected_checksum, before_image, writes) before modifying any artifact.
2. **Apply writes** — Apply `CommandResult.writes` in order.
3. **Write manifest** — Set `checksum` to the new value and `previous_checksum` to the old; this is the commit point. Done via `ManifestStore::write`.
4. **Delete journal** — Clean up.

Recovery on open: if `.journal` exists, recompute actual checksum and follow the spec’s Case A/B/C (rewrite manifest, discard journal, or undo from before_image). `ManifestStore` is the port over which the snapshot (checksum chain) is read and written so this protocol stays inside the hexagon boundary.

### Composition boundary (`VaultPorts` + `UseCases`)

Dependencies are injected at a single boundary, analogous to `createUseCases({ databasePort, authPort, ... })` in a TS/Express bootstrap:

- **`VaultPorts<'a>`** — Struct holding references to all five port traits (blocks, graph, documents, names, manifest). Built once at the composition root (CLI, server, WASM adapter, or test).
- **`UseCases::new(ports)`** — Returns the use-case surface with ports injected. Call `use_cases.add_block(...)`, `use_cases.rename_block(...)`, etc. No use case receives more than one injected value; the adapter passes `&ports` (or a `UseCases` built from it).

The composition root is the only place that wires concrete adapters into `VaultPorts` and constructs `UseCases`. Unit tests continue to call individual `use_cases::add_block::execute(&mock_blocks, &mock_names, ...)` with mocks; integration and E2E tests can build `VaultPorts` from real or in-memory stores and use `UseCases` for a single, mockable boundary.

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

## Open, validate, and full vault

**Checksum-first validation (per request).** Operations stay atomic and idempotent: the gate is a checksum check, not “validate entire vault before any operation.” The adapter (or composition root) does the following before permitting a mutation: (1) **Checksums match** — computed checksum equals manifest’s `checksum` → no obstacle; proceed. (2) **Checksums mismatch** — run full `validate_vault(vault)`. If the result is OK (e.g. drift is benign or explained), no obstacle; proceed. (3) **Revalidation not OK** — remediation required: either enrich the spec to handle the case or require human input; do not proceed until resolved. The core provides `checksum::compute(vault)`, `checksum::is_drifted(vault)`, and `invariants::validate_vault(vault)`; the adapter owns policy. Requests that follow closely after a successful commit may skip the checksum check (state is known current), but implementations may choose to always check for consistency.

**Full vault for validate and checksum.** Both validation and checksum need a full in-memory `Vault` (manifest, blocks, graph, documents, names). The core has `checksum::compute(vault)` and no I/O. So the adapter must be able to produce a `Vault` — either by reading through the five read ports and building the struct, or by a dedicated “load vault from path” path used at open and (if needed) after apply_writes to compute the new checksum for the manifest. Bootstrapping a new vault is the case where the vault is empty and validation passes trivially.

**Composition root naming.** The object that holds open adapters and exposes the use-case surface is the composition root for one open vault. Do not call it a “session.” Session implies a continuous connection — something that is established, sustained, and later closed — and that assumption is wrong: the vault is a directory; there is no inherent connection or lifecycle. A process opens it (load, validate, wire adapters), runs commands, and may hold the handle for one command or many. Another process could open the same vault. Naming it a session encourages the wrong mental model (e.g. exclusive ownership, session-scoped state, or a link that must be kept alive). Use names that reflect a handle or context for an open vault, e.g. `VaultHandle` or `OpenVault`. The important invariant is “validate (and optionally drift-check) the loaded vault before allowing mutations.”

---

## Port failure and I/O errors

Port traits currently return `Option<T>` or `Vec<T>`, not `Result<T, E>`. So when an adapter hits an I/O error (e.g. disk full, permission denied), it cannot propagate it; in practice adapters use `.expect()` and panic, or (e.g. on block parse) skip and log. The **intended policy** when a port “fails” is owned by the composition root: **rewind** (if journal and before_image exist), **retry**, or **panic**; user intervention (e.g. “fix disk and retry”) is also a valid outcome. If port methods are later extended to return `Result`, the domain still does not see I/O; the use case or the composition root would handle the error and decide rewind/retry/panic/user. This is deliberately underspecified in the core so adapters can choose their own durability and error-surfacing strategy.

---

## Client Separation

The core is a library crate. It has no knowledge of transport, UI framework, or platform. Clients are separate projects that depend on the core:

- **Desktop** — A Tauri shell that runs the core in-process and bridges commands/events via IPC. The frontend is a SolidJS app.
- **Web** — A server binary that exposes the core over HTTP. The same SolidJS frontend talks to it via fetch.
- **CLI / other** — Any binary that depends on `portablenote-core` and wires up adapters.

Tauri is one way to build a desktop client. It is not required by the spec and not the only option.

---

## Consuming the core from TypeScript / TSX

Goal: a **simple, stable, compliant API** that a TS/TSX app can call without caring whether the core runs in the same process (WASM), in a desktop shell (Tauri), or on a server (HTTP).

### One API surface, multiple delivery mechanisms

Define a single, serializable contract that all delivery mechanisms implement:

- **Input:** vault snapshot (JSON) + command (JSON). The vault is the full in-memory state (manifest, blocks, graph, documents, names); the host is responsible for loading it (from disk, fetch, or in-memory).
- **Output:** `{ writes: VaultWrite[], event: <event> }` or a rejection with a stable error code/message. All types already have `serde::Serialize` / `Deserialize` in the core.

The TS layer then only ever does: pass vault + command in, get writes + event (or error) back, and applies writes however it wants (persist to disk, send to server, update local state). No Rust types leak; no I/O in the core call.

### Recommended delivery for “easy TS consumption”

| Mechanism | Best for | How TS consumes |
|-----------|----------|------------------|
| **WebAssembly (WASM)** | Browser, Node, or “one binary” shared logic | `import { execute, validate, computeChecksum } from 'portablenote-wasm'` (or similar). Same package works in browser and Node. No server required; offline-first. |
| **Tauri** | Desktop app (TS frontend + Rust process) | No WASM in the frontend. Expose Tauri commands that mirror the same API (e.g. `invoke('execute', { vault, command })`). Core runs as a Rust library; TS gets the same JSON-in/JSON-out contract over IPC. |
| **HTTP** | Server-backed app (vault on server) | TS does `fetch('/api/execute', { body: JSON.stringify({ command }) })`. Server loads vault from disk, runs core, applies writes, returns result. Easiest integration; no Rust or WASM in the TS bundle. |

**Recommendation:** Use **WASM** as the primary path for a single, stable TS API when you want the same logic in browser and Node with one dependency. Use **Tauri** when the client is a desktop app (same core, no WASM; expose the same logical API as Tauri commands). Use **HTTP** when the vault lives on a server and the frontend is a thin client.

### What the core needs for a “driver” API

The use cases today take **ports** (e.g. `BlockStore`, `GraphStore`). To expose a single “execute(vault, command)” style API (for WASM, or for a small server that holds no persistent state between calls), you need a **driver** that:

1. Takes a `Vault` snapshot and a command (e.g. “AddBlock” with payload).
2. Builds **in-memory stores** from the vault (same pattern as `core/tests/support/factory::from_vault`).
3. Runs the appropriate use case against those stores (and a clock).
4. Returns the `CommandResult` (writes + event) or `DomainError`, all serializable.

So: either move or re-export the in-memory store implementations so they are part of the library (not only test code), and add a small **driver** module (in core or in a dedicated crate such as `portablenote-api` or `portablenote-wasm`) that exposes a handful of functions:

- `execute(vault: Vault, command: Command) -> Result<CommandResult, DomainError>`
- `validate(vault: Vault) -> Vec<Violation>`
- `compute_checksum(vault: Vault) -> String`

For WASM, that driver is compiled to WASM; the host (TS) passes vault and command as JSON (or typed structs that serialize to the same shape). No file I/O inside WASM — the host is responsible for persistence. For Tauri, the same driver runs in Rust; the Tauri command layer calls it and returns the serialized result to the frontend. Same contract, same compliance; only the delivery mechanism changes.

---

## Design Decisions

### DeleteBlockCascade updates documents

The two delete modes exist so the user can choose: **Safe** (fail if anything references the block) or **Cascade** (remove the block and clean up all references). When they choose cascade, they really want the block gone — so we remove it from documents too, not leave dangling section/subsection references.

Cascade delete therefore:
- Removes the block from every document that references it: if the block is a **section**, that section is removed; if it is a **subsection**, that subsection is removed; if the block is a document **root**, that document is deleted (it has no valid root).
- Emits the corresponding `WriteDocument` / `DeleteDocument` writes so the vault stays consistent and validation does not flag dangling UUIDs.
