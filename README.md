# PortableNote

**Portability equals ownership.**

PortableNote is a directory-based knowledge management format where the block graph is the primary artifact and documents are optional views over it. No platform holds the authoritative copy. No special tooling is required to read your data. Any conforming implementation can open any conforming vault with full fidelity.

## Repository Structure

```
portablenote/
  spec/                  Format specification, JSON schemas, compliance test suite
  core/                  Reference implementation in Rust (portablenote-core)
  infra/                 Infrastructure adapters (portablenote-infra)
  cli/                   Command-line interface (pn)
```

### `spec/`

The implementation-agnostic format specification. Defines vault structure, artifact schemas, mutation commands, validation invariants, and behavioral contracts. Any language can implement against this spec and run the compliance suite.

See [`spec/README.md`](spec/README.md) for details.

### `core/`

The reference Rust implementation. A hexagonal-architecture domain library with pure functions, port traits, and use cases. No I/O in the domain layer; infrastructure adapters live outside this crate.

See [`core/ARCHITECTURE.md`](core/ARCHITECTURE.md) for the implementation's design.

### `infra/`

Filesystem adapters that implement the port traits from `core`. Each vault artifact maps to files on disk: blocks as `.md` files, the graph as `block-graph.json`, documents as individual JSON files, and the name index as `names.json`.

### `cli/`

The `pn` command-line tool for local vault management. Supports `init`, `add`, `rename`, `edit`, `delete`, `link`, `unlink`, and `list` commands. Uses `infra` adapters to persist changes to disk.

## Key Concepts

- **Blocks** are named semantic units. A block is as large or small as the idea requires. Its name is its identity in the link system.
- **The graph is the knowledge.** Typed, directed edges between blocks capture relationships explicitly. The graph is a first-class artifact, not rebuilt by scanning.
- **Documents are views.** An ordered composition of blocks from the heap. The same block may appear in many documents. Rearranging documents never affects the graph.
- **Links are live.** Inline references use human-readable names. A rename propagates everywhere automatically.
- **Multiple export formats.** Source content is Markdown. Rendered output can target `.md`, `.rtf`, `.docx`, and others.

## Status

Pre-RFC draft. The spec and reference implementation are under active development. The compliance suite covers vault validation, invariants, and all 19 mutation scenarios.

## License

Apache 2.0
