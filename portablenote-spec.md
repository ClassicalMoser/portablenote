# PortableNote Format Specification
**Version:** 0.1.0-draft  
**License:** Apache 2.0  
**Status:** Pre-RFC Draft

---

## Philosophy

Portability equals ownership. A PortableNote vault is a directory of plain files. No platform holds the authoritative copy. No special tooling is required to read your data. Any conforming implementation can open any conforming vault with full fidelity.

The spec is the contract. The tool is a proof of concept.

---

## Core Principles

- **Composability over inheritance.** Types fulfill contracts via traits, not hierarchies.
- **Separation of concerns.** The heap owns content. The composition tree owns arrangement. The reference graph owns meaning.
- **Explicit over derived.** The graph is a first-class artifact, not rebuilt by scanning.
- **Validation at every mutation.** Invariants hold after every transaction, not eventually.
- **Git is version control.** Content history is delegated to git or equivalent. Format versioning is handled by the manifest.
- **Markdown is an adapter.** Not the foundation. The first and most important adapter, but one of many.

---

## Vault Structure

A vault is a directory with the following layout:

```
/vault
  /blocks          # Flat directory of block files, UUID-named
  manifest.json    # Vault identity, version, format declaration, checksum
  tree.json        # Composition tree(s) — ordered node hierarchies
  graph.json       # Reference graph — typed directed edges
```

No other structure is required or assumed. The program validates all four artifacts on open and rejects or remediates inconsistencies.

---

## 1. Manifest (`manifest.json`)

Declares vault identity, spec version, content format, and integrity checksum.

### Schema

```json
{
  "vault_id": "uuid-v4",
  "spec_version": "0.1.0",
  "format": "markdown",
  "checksum": "sha256:<hex>"
}
```

### Fields

| Field | Type | Description |
|---|---|---|
| `vault_id` | UUID v4 | Permanent vault identity. Never changes. |
| `spec_version` | semver string | PortableNote spec version this vault conforms to. |
| `format` | string | Content format for all blocks in this vault. `"markdown"` for v0. Extensible. |
| `checksum` | string | SHA-256 over canonical serialization of `tree.json`, `graph.json`, and sorted block file hashes. Prefixed `sha256:`. |

### Checksum Computation

```
checksum = sha256(
  canonical_json(tree.json) +
  canonical_json(graph.json) +
  sorted([sha256(block_file) for each file in /blocks])
)
```

Canonical JSON: keys sorted alphabetically, no whitespace. On open, the program recomputes and compares. Mismatch triggers a validation pass and re-sign if the vault is consistent. Mismatch is advisory, not blocking — the user retains full control.

---

## 2. Blocks (`/blocks`)

The heap. Every block is a file in `/blocks`, named by its UUID with a format extension.

### Naming Convention

```
/blocks/<uuid>.<ext>
```

For the Markdown format: `/blocks/a3f9b2c1-....md`

### Block Frontmatter

Every block file begins with a YAML frontmatter header:

```yaml
---
id: <uuid-v4>
type: <block_type>
contract: <contract_name>   # optional
created: <iso8601>
modified: <iso8601>
---
```

Content follows immediately after the closing `---`.

### Fields

| Field | Required | Description |
|---|---|---|
| `id` | Yes | UUID v4. Permanent. Never changes. Must match filename. |
| `type` | Yes | Block type. Core types defined below. Extensible. |
| `contract` | No | Optional contract this block fulfills (e.g. `meeting_note`, `article`). |
| `created` | Yes | ISO 8601 creation timestamp. |
| `modified` | Yes | ISO 8601 last modification timestamp. Updated on every content mutation. |

### Core Block Types

| Type | Description |
|---|---|
| `paragraph` | Standard prose content. |
| `code` | Code block. Requires `language` frontmatter field. |
| `list` | Ordered or unordered list. |
| `quote` | Block quotation. |
| `callout` | Highlighted or annotated content. |
| `image` | Image reference. Requires `src` frontmatter field. |
| `embed` | Embedded reference to another block or external resource. |

Block types are extensible. Implementations may define additional types. Unknown types must be preserved on round-trip and treated as opaque content.

### Markdown Content Rules

When `format` is `"markdown"`:

- Content is CommonMark compliant.
- **No headings permitted inside blocks.** Headings are structural, not content. They belong to the composition tree.
- Inline formatting (bold, italic, code spans, links) is permitted.
- Fenced code blocks are permitted inside `code` typed blocks.
- No h1–h6 syntax inside block content. Violation is a parse error.

### Inline Block References

A block may reference another block inline using the following syntax:

```markdown
See also [[uuid:a3f9b2c1-...]] for more context.
```

Inline references are living — they resolve at read time against the current heap. A reference to a deleted block renders as a broken reference indicator. Inline references within block content reference **blocks only**, never composition nodes. This is enforced at parse time.

### Footer Reference Declarations

Outgoing reference edges from a block to other blocks may be declared explicitly in a footer section, separate from inline syntax. This is the bridge between block content and the reference graph.

```markdown
---
id: <uuid>
type: paragraph
---

Content here with an [[uuid:a3f9b2c1-...]] inline reference.

<!-- refs -->
- elaborates: uuid:b4e8d3f2-...
- contradicts: uuid:c7a1e9d4-...
- categorizes: uuid:d2f6b8a5-...
```

Footer declarations are parsed by conforming implementations and used to populate `graph.json`. They are redundant with the graph but serve as a human-readable and git-diffable record of outgoing edges from that block.

---

## 3. Reference Graph (`graph.json`)

The semantic layer. Typed directed edges between any two node UUIDs.

### Schema

```json
{
  "version": "0.1.0",
  "edges": [
    {
      "id": "uuid-v4",
      "source": "uuid-v4",
      "target": "uuid-v4",
      "edge_type": "references"
    }
  ]
}
```

### Edge Fields

| Field | Type | Description |
|---|---|---|
| `id` | UUID v4 | Permanent edge identity. |
| `source` | UUID v4 | Source node. Must exist in heap or composition tree. |
| `target` | UUID v4 | Target node. Must exist in heap or composition tree. |
| `edge_type` | string | Typed relationship. Core types below. Extensible. |

### Core Edge Types

| Type | Description |
|---|---|
| `references` | General reference. Default. |
| `elaborates` | Source expands on target. |
| `contradicts` | Source disputes or negates target. |
| `categorizes` | Source categorizes or tags target. |
| `derived_from` | Source was forked or inspired by target. |
| `depends_on` | Source has a dependency on target. |

Edge types are extensible. Implementations may define additional types. Unknown types must be preserved.

### Reference Rules

These invariants are enforced at every mutation:

1. **Block → Block.** A block may reference any other block freely. Cross-vault, cyclic, unrestricted.
2. **Composition node → same-composition node.** A composition node may reference any other node within the same composition tree.
3. **Composition node → external root only.** A composition node may reference only the root node of a different composition tree. Not internal nodes of another composition.
4. **Block → composition node: forbidden.** A block may not directly reference a composition node. Blocks are compositionally agnostic.

Violations are rejected at mutation boundary.

### Reference Graph Properties

- Freely cyclic. Cycles are valid and expected.
- No acyclicity constraint.
- Order is irrelevant. The edge list is unordered.
- Referential integrity: every UUID in `source` or `target` must exist in the heap or composition tree. Dangling UUIDs are a validation error.

---

## 4. Composition Tree (`tree.json`)

The arrangement layer. Ordered hierarchies of nodes over the heap. Multiple independent compositions are supported over the same heap.

### Schema

```json
{
  "version": "0.1.0",
  "compositions": [
    {
      "id": "uuid-v4",
      "name": "My Notes",
      "root": "uuid-v4",
      "nodes": [
        {
          "id": "uuid-v4",
          "node_type": "document",
          "title": "Getting Started",
          "children": ["uuid-v4", "uuid-v4"],
          "references": [
            { "target": "uuid-v4", "ref_type": "internal" },
            { "target": "uuid-v4", "ref_type": "external_root" }
          ]
        }
      ]
    }
  ]
}
```

### Composition Fields

| Field | Type | Description |
|---|---|---|
| `id` | UUID v4 | Permanent composition identity. |
| `name` | string | Human-readable composition name. |
| `root` | UUID v4 | UUID of the root composition node. |
| `nodes` | array | All composition nodes in this composition. |

### Composition Node Fields

| Field | Type | Description |
|---|---|---|
| `id` | UUID v4 | Permanent node identity. |
| `node_type` | string | `document` or `section`. Extensible. |
| `title` | string | Human-readable title. Rendered as heading by adapters. |
| `children` | array of UUID | Ordered child UUIDs. May be block UUIDs or composition node UUIDs. |
| `references` | array | Outgoing composition-level references. Internal or external root only. |

### Composition Node Types

| Type | Description |
|---|---|
| `document` | A named, navigable document. Flat — no document nesting. |
| `section` | A named section within a document. One level of subsections permitted. |

### Composition Properties

- **Flat documents.** Documents do not nest within documents. Hierarchy between documents is expressed as typed reference edges.
- **Sections and subsections only.** Two levels of intra-document structure. Deeper structure is a new document with a reference edge.
- **Non-exclusive membership.** A block UUID may appear in the `children` list of multiple composition nodes across multiple compositions. The heap owns the block. Compositions arrange it.
- **Acyclic.** A composition node may not be its own ancestor. Circular composition is a validation error.
- **Multi-composition.** A vault may contain multiple independent composition trees over the same heap. Each has its own UUID, name, and root.

### Orphaned Blocks

A block that does not appear in any composition node's `children` list is an orphan. Orphans are valid — the heap owns them. Conforming implementations surface orphans in a "heap browser" or equivalent UI, making all blocks accessible before organization is complete. Orphaned blocks may be referenced by other blocks via the reference graph.

---

## 5. CQRS Mutation Standards

All state changes are commands. Queries never mutate state. Validation occurs before commitment. Failed commands are rejected with a descriptive error. Successful commands update the relevant artifact(s) and recompute the checksum.

### Commands

#### Block Commands

| Command | Description | Validates |
|---|---|---|
| `AddBlock` | Add a new block file to `/blocks`. | UUID unique, type valid, frontmatter complete. |
| `MutateBlockContent` | Update block content. Updates `modified` timestamp. | Block exists, content valid for declared format. |
| `DeleteBlock(safe)` | Delete block. Fails if incoming reference edges exist. | No incoming edges in `graph.json`. |
| `DeleteBlock(cascade)` | Delete block. Removes all incoming reference edges first. Emits warning with count. | Block exists. |

#### Composition Commands

| Command | Description | Validates |
|---|---|---|
| `AddCompositionNode` | Add a document or section node to a composition. | UUID unique, parent exists, acyclicity preserved. |
| `AppendChild` | Add a UUID to a node's children list. | Parent exists, child UUID exists in heap or tree, no acyclicity violation. |
| `RemoveChild` | Remove a UUID from a node's children list. | Parent exists, child present in list. |
| `ReorderChildren` | Reorder a node's children list. | Same UUIDs, different order. |
| `DeleteCompositionNode(safe)` | Delete node. Fails if incoming reference edges exist. | No incoming edges. |
| `DeleteCompositionNode(cascade)` | Delete node. Removes incoming edges. Emits warning. | Node exists. |

#### Reference Commands

| Command | Description | Validates |
|---|---|---|
| `AddEdge` | Add a typed edge to `graph.json`. | Source and target exist, reference rules satisfied, edge type valid. |
| `RemoveEdge` | Remove an edge by edge UUID. | Edge exists. |
| `MutateEdgeType` | Change the type of an existing edge. | Edge exists, new type valid. |

### Events

Every successful command emits a domain event. Events are consumed by the UI layer via Tauri event bridge. Events are not persisted in v0.

| Event | Payload |
|---|---|
| `BlockAdded` | block UUID, type |
| `BlockContentMutated` | block UUID |
| `BlockDeleted` | block UUID, edges_removed count |
| `CompositionNodeAdded` | node UUID, composition UUID |
| `ChildAppended` | parent UUID, child UUID |
| `ChildRemoved` | parent UUID, child UUID |
| `ChildrenReordered` | parent UUID |
| `CompositionNodeDeleted` | node UUID, edges_removed count |
| `EdgeAdded` | edge UUID, source, target, type |
| `EdgeRemoved` | edge UUID |
| `VaultOpened` | vault UUID, checksum_status |
| `ChecksumMismatch` | expected, actual, drift_summary |

---

## 6. Validation Invariants

These invariants must hold after every mutation. Conforming implementations enforce all of them.

1. Every UUID in `graph.json` source or target fields exists in the heap or composition tree.
2. Every UUID in `tree.json` children arrays exists in the heap or composition tree.
3. No composition node is its own ancestor (acyclicity).
4. No block references a composition node (enforced at `AddEdge` and inline reference parse time).
5. Cross-composition references target only root nodes.
6. Every block file UUID matches its frontmatter `id` field.
7. No block content contains heading syntax (h1–h6) when format is `"markdown"`.
8. The manifest checksum reflects the current state of all artifacts.

---

## 7. Import Standard (Markdown Vaults)

Conforming implementations should support import from existing Markdown vaults (Obsidian, Logseq export, plain `.md` directories).

### Heading Promotion Rules

On import of an existing `.md` file:

- **h1** becomes the document title.
- **h2** becomes a section node.
- **h3** becomes a subsection node.
- **h4 and below** trigger creation of a new document node. A `references` edge of type `elaborates` is created from the parent section to the new document root. This is lossy but correct — deep hierarchy becomes graph structure.

### Wikilink Conversion

`[[page name]]` wikilinks are resolved to block or document UUIDs where possible. Unresolvable wikilinks are preserved as plain text with a warning.

### Frontmatter Mapping

Existing YAML frontmatter fields are preserved as block metadata. Unknown fields are kept verbatim.

---

## 8. Export Standard

A conforming implementation must be able to export any composition as a Markdown library — a directory of readable `.md` files reflecting the composition structure.

### Export Layout

```
/export/<composition-name>/
  <document-title>.md
  <document-title>/
    <section-title>.md
```

### Export Rules

- Document title becomes h1 in the exported file.
- Section titles become h2. Subsection titles become h3.
- Block content is concatenated in composition order.
- Inline block references render as wikilinks: `[[uuid:...]]` or resolved titles where available.
- Reference graph edges are lost on export. This is expected and acceptable — Markdown export is for readability and interop, not round-trip fidelity.
- Export may be live — re-run automatically on every mutation. Conforming implementations may offer a watched export mode.

---

## 9. Contract System

Composition nodes and blocks may declare a contract — a named type that implies structural or content expectations. Contracts are fulfilled via trait implementations, not inheritance.

### Contract Declaration

In block frontmatter:
```yaml
contract: meeting_note
```

In composition node:
```json
{ "contract": "article" }
```

### Core Contracts (v0)

| Contract | Applies to | Implied structure |
|---|---|---|
| `article` | Document | Has introduction section. Export as single document. |
| `meeting_note` | Document | Has date, attendees, action items sections. |
| `daily_note` | Document | Has date. One per day convention. |
| `reference` | Block | Bibliographic or external reference content. |

Contracts are extensible. Unknown contracts are preserved and ignored by implementations that do not implement them.

---

## 10. Adapter Interface

The content format adapter is a port. The Markdown adapter is the v0 reference implementation. Future adapters (RTF, HTML, Portable Text) implement the same port.

### Port Contract

A conforming adapter must implement:

- `parse(file: &Path) -> Result<BlockContent>` — reads a block file, returns domain content.
- `serialize(content: &BlockContent, file: &Path) -> Result<()>` — writes domain content to file.
- `validate_content(content: &str) -> Result<()>` — validates raw content string against format rules.
- `extract_inline_refs(content: &str) -> Vec<Uuid>` — returns all inline block reference UUIDs.

### Format Declaration

Format is declared once per vault in the manifest. All blocks in a vault use the same format. Format migration between vaults requires re-serialization through the adapter port. Mixing formats within a vault is not permitted in v0.

---

## 11. DDD / Hexagonal Layer Separation

The reference implementation follows strict hexagonal architecture. Layer boundaries are enforced — inner layers have no dependencies on outer layers.

```
┌─────────────────────────────────────────┐
│                  UI Layer               │  SolidJS via Tauri event bridge
├─────────────────────────────────────────┤
│             Application Layer           │  CQRS handlers, validation orchestration
├─────────────────────────────────────────┤
│               Domain Layer              │  Pure Rust. Zero external dependencies.
├─────────────────────────────────────────┤
│            Infrastructure Layer         │  Adapters: filesystem, Markdown, search
└─────────────────────────────────────────┘
```

### Domain Layer (Pure Rust Crate)

- Defines all core types: `Block`, `CompositionNode`, `Composition`, `Edge`, `Vault`, `Heap`.
- Defines all commands and queries.
- Defines all invariants as pure functions.
- No serde, no filesystem, no Markdown, no async. Pure domain logic only.
- Publishable as a standalone crate. Other implementations depend on it directly or reimplement the same contract.

### Application Layer

- CQRS command handlers. Each command: validate → mutate → emit event.
- Query handlers. Read-only. No mutation.
- Owns the transaction boundary — all artifact mutations in a command are atomic.
- Depends on domain layer and port definitions only.

### Port Definitions

Defined in the application layer. Implemented in infrastructure.

- `ContentFormatPort` — parse, serialize, validate, extract refs.
- `PersistencePort` — read vault, write artifacts, list blocks.
- `SearchPort` — full text search within a bounded block set.
- `ExportPort` — serialize composition to external format.

### Infrastructure Layer

- `MarkdownAdapter` — implements `ContentFormatPort`. Depends on `pulldown-cmark`.
- `FilesystemAdapter` — implements `PersistencePort`. Reads and writes vault directory.
- `SearchAdapter` — implements `SearchPort`. Simple text search for v0.
- `ExportAdapter` — implements `ExportPort`. Markdown library export.

### UI Layer

- SolidJS frontend.
- Communicates exclusively via Tauri commands (CQRS commands) and Tauri events (domain events).
- Never touches the filesystem directly.
- Never knows about UUIDs except as opaque strings passed through.
- All intelligence lives in Rust.

---

## 12. Scaffolding Outline

```
portablenote/
  spec/                        # The specification (this document + schemas)
    portablenote-spec.md
    schemas/
      manifest.schema.json
      tree.schema.json
      graph.schema.json
    compliance/                # Compliance test suite
      valid/                   # Valid vault snapshots
      invalid/                 # Invalid vault snapshots  
      mutations/               # Mutation scenarios with expected outcomes

  crates/
    portablenote-domain/       # Pure domain layer. Zero dependencies.
      src/
        lib.rs
        types/
          block.rs
          composition.rs
          edge.rs
          heap.rs
          vault.rs
        commands/
          block_commands.rs
          composition_commands.rs
          edge_commands.rs
        queries/
          block_queries.rs
          composition_queries.rs
          graph_queries.rs
        invariants.rs
        events.rs

    portablenote-app/          # Application layer. CQRS handlers.
      src/
        lib.rs
        ports/
          content_format.rs
          persistence.rs
          search.rs
          export.rs
        handlers/
          command_handlers.rs
          query_handlers.rs
        validation.rs

    portablenote-infra/        # Infrastructure. Adapters.
      src/
        lib.rs
        adapters/
          markdown/
            mod.rs
            parser.rs          # pulldown-cmark integration
            serializer.rs
            ref_extractor.rs
          filesystem/
            mod.rs
            vault_reader.rs
            vault_writer.rs
            checksum.rs
          search/
            mod.rs
          export/
            mod.rs
            markdown_export.rs

    portablenote-tauri/        # Tauri shell. Command/event bridge.
      src/
        main.rs
        commands.rs            # Tauri command handlers → app layer
        events.rs              # Domain events → Tauri emit

  ui/                          # SolidJS frontend
    src/
      App.tsx
      components/
        BlockEditor/
        CompositionView/
        HeapBrowser/
        GraphView/
        SearchBar/
      stores/                  # Reactive state
      bridge/                  # Tauri invoke wrappers

  README.md
  LICENSE                      # Apache 2.0
```

---

## Appendix: Open Questions for v0.2+

- Block content mutability: can a block's type change after creation?
- Multi-vault references: can a block in vault A reference a block in vault B?
- Contract validation strictness: warning or error on contract violation?
- Search adapter: regex, fuzzy, or exact only for v0?
- Live export: push or poll model for watched export?
- Performance targets: expected block count ceiling for v0?
- Conflict resolution: out of scope, placeholder for future spec revision.
- Compliance certification: informal for v0, registry model for v1+.

---

*PortableNote Specification v0.1.0-draft. Subject to revision. Portability equals ownership.*
