# PortableNote Format Specification
**Version:** 0.1.0-draft  
**License:** Apache 2.0  
**Status:** Pre-RFC Draft

---

## Philosophy

Portability equals ownership. A PortableNote vault is a directory of plain files. No platform holds the authoritative copy. No special tooling is required to read your data. Any conforming implementation can open any conforming vault with full fidelity.

The block graph is the knowledge base. Documents are optional views over it. A vault with no documents is complete. A vault with no graph is just files.

The spec is the contract. The tool is a proof of concept.

---

## Core Principles

- **The graph is the knowledge.** The block graph is the primary artifact. Documents are derived views, not the source of truth.
- **Blocks are named semantic units.** A block is author-bounded — as large or small as the idea requires. Its name is its identity in the link system.
- **Headings are boundaries, not content.** A heading encountered in content ends the current block and begins a new one. Heading level is a rendering concern, determined by document context, not baked into content.
- **Links are live.** Inline references use human-readable names. The graph watches its members. A rename propagates everywhere automatically.
- **Documents are views.** A document is an ordered composition of blocks from the heap. The same block may appear in many documents. Rearranging documents never affects the block graph.
- **Explicit over derived.** The graph is a first-class artifact, not rebuilt by scanning.
- **Validation at every mutation.** Invariants hold after every transaction, not eventually.
- **Git is version control.** Content history is delegated to git or equivalent. Format versioning is handled by the manifest.
- **Markdown is an adapter.** Not the foundation. The first and most important adapter, but one of many.

---

## Vault Structure

A vault is a directory with the following layout:

```
/vault
  /<composition-name>/       # Rendered .md document trees — one per composition
  /<composition-name>/       # Multiple compositions supported

  /portablenote/             # Source artifacts (canonical data)
    manifest.json            # Vault identity, version, format declaration, checksum chain
    names.json               # Name-to-UUID index (derived from block metadata)
    /blocks                  # Primary — heap of named block files
    block-graph.json         # Primary — typed directed edges between blocks
    /documents               # Optional — one JSON definition file per document composition
    .journal                 # Ephemeral — present only during an in-flight commit (see §5a)
```

A user opening the vault sees readable, named document trees at the root. The `portablenote/` directory contains all source artifacts — visible, portable, and inspectable without tooling.

All source artifact paths in this spec are relative to `portablenote/` unless otherwise noted.

The `/blocks` directory and `block-graph.json` are the canonical knowledge base. The `/documents` directory is optional — a vault with no documents is complete and fully navigable via the graph. Rendered output trees are derived and rebuilt on any mutation. They are never edited directly.

A conforming implementation enforces the mutation gate (§5) before permitting any mutation: checksum check, then full validation on mismatch; remediation is required when validation fails. Rendered trees are not validated — they are regenerated.

---

## 1. Manifest (`manifest.json`)

Declares vault identity, spec version, content format, and integrity checksum.

### Schema

```json
{
  "vault_id": "uuid-v4",
  "spec_version": "0.1.0",
  "format": "markdown",
  "checksum": "sha256:<hex>",
  "previous_checksum": "sha256:<hex>" | null
}
```

### Fields

| Field | Type | Description |
|---|---|---|
| `vault_id` | UUID v4 | Permanent vault identity. Never changes. |
| `spec_version` | semver string | PortableNote spec version this vault conforms to. |
| `format` | string | Content format for all blocks in this vault. `"markdown"` for v0. Extensible. |
| `checksum` | string | SHA-256 over canonical serialization of blocks, edges, and documents. Prefixed `sha256:`. |
| `previous_checksum` | string \| null | Checksum of the vault state before the most recent commit. `null` for the genesis commit (vault init). Together with `checksum`, forms a hash chain: each commit is a verifiable `(before, after)` state transition. Two manifests sharing a `previous_checksum` but differing on `checksum` indicate a fork. |

### Checksum Computation

The checksum is a SHA-256 hash over a canonical byte representation of blocks, edges, and documents. The manifest itself is not included — its fields (`vault_id`, `spec_version`, `format`) are identity/config that do not represent mutable content. The `names.json` index is also excluded — it is derived from block metadata and can be reconstructed by scanning `/blocks`. Canonical serialization concatenates the following, in order:

1. **Blocks**, sorted by UUID (lexicographic on the hyphenated string). Each block contributes:
   ```
   block:<uuid>\n<name>\n<content>\n
   ```
2. **Edges**, sorted by edge UUID. Each edge contributes:
   ```
   edge:<uuid>\n<source>-><target>\n
   ```
3. **Documents**, sorted by document UUID. Each document contributes:
   ```
   doc:<uuid>\nroot:<root_uuid>\n
   ```
   followed by sections in their declared order (order is semantically significant — not sorted):
   ```
   section:<block_uuid>\n
   ```
   and for each subsection:
   ```
   sub:<block_uuid>\n
   ```

If `/documents` is empty, documents contribute nothing. Block timestamps are excluded — only identity, name, and content participate.

#### Normalization Rules

All conforming implementations must apply the following rules identically to produce interoperable checksums:

| Field | Rule |
|---|---|
| All UUIDs | Lowercase, hyphenated: `a3f9b2c1-0000-4000-a000-000000000001` |
| `name` and all string fields | UTF-8, NFC-normalized |
| Line endings in `content` | LF (`\n`) only. `\r\n` is normalized to `\n` on write; `\r\n` in stored content is a format violation. |
| `content` | Hashed as-stored. No trimming, no padding. The stored bytes are the canonical bytes. |

These rules ensure that any conforming implementation — regardless of language or platform — produces identical checksums for the same logical vault state.

The result is stored as `sha256:<hex>`. If `.journal` is present on open, a checksum mismatch triggers the recovery protocol (see §5a). When no journal is present, checksum mismatch triggers the mutation gate (see §5 Mutation gate): the implementation runs full validation; if validation passes, mutation is permitted (and the manifest may be updated to reflect current state); if validation fails, remediation is required before any mutation.

---

## 1a. Names Index (`names.json`)

The vault-wide name → UUID lookup. A peer artifact to `block-graph.json` and `/documents`, stored as a sibling of the manifest.

### Schema

```json
{
  "Getting Started": "uuid-v4",
  "Key Insight": "uuid-v4"
}
```

A plain JSON object mapping every block `name` (string) to its UUID (string). Updated on every `AddBlock`, `RenameBlock`, and `DeleteBlock`.

The names index is derived from block metadata and can be reconstructed by scanning `/blocks` if needed. It is not included in the checksum computation.

---

## 2. Blocks (`/blocks`)

The heap. Every block is a file in `/blocks`, named by its human-readable name with a format extension. Blocks are the primary entities of the vault — the named semantic units from which all knowledge is built.

### What Is a Block

A block is an **author-bounded semantic unit**. It is as large or small as the idea it expresses requires — a single sentence, several paragraphs, a code snippet with surrounding explanation. The author decides where a block begins and ends. A carriage return does not create a new block. **A heading does.**

When a heading is encountered during parsing, it ends the current block and begins a new one. The heading text becomes the new block's `name`. This is the only block boundary mechanism.

### Filename Convention

Block files are named by their human-readable `name`, percent-encoded for filesystem safety, with a format extension:

```
/blocks/<percent-encoded-name>.<ext>
```

For example: `/blocks/Getting Started.md`, `/blocks/Café Culture.md`, `/blocks/Notes%3A Part 1.md`

#### Percent-Encoding Algorithm

The filename is derived from the block's `name` metadata field using RFC 3986 percent-encoding over the following restricted character set:

| Encode | Characters |
|---|---|
| Filesystem-unsafe | `/ \ : * ? " < > \|` |
| Control characters | U+0000–U+001F, U+007F |
| Percent literal | `%` (to avoid ambiguity) |

All other characters — including spaces, unicode, and common punctuation — pass through unmodified. Unicode is normalized to NFC before encoding.

The filename is a **derived projection** of the metadata `name`, not the source of truth. The metadata in the HTML comment header is always authoritative. If a filename does not match `encode(metadata.name) + extension`, the implementation corrects the filename on open.

#### Filename Length

Encoded filenames are truncated to 200 bytes (UTF-8). Names that exceed this after encoding are truncated and disambiguated with a numeric suffix: `Very Long Name That Exceeds... (2).md`. This limit accommodates all major filesystem path component limits (255 bytes on ext4, NTFS, HFS+) with room for the extension.

### Block Metadata

Every block file begins with a metadata header inside an HTML comment. This ensures metadata is invisible in any CommonMark-compliant renderer — a block file opened in any markdown viewer shows only the author's content.

```markdown
<!--
id: <uuid-v4>
name: <human-readable name>
created: <iso8601>
modified: <iso8601>
-->
```

The metadata is YAML inside an HTML comment. Content follows immediately after the closing `-->`. Additional metadata fields are permitted and preserved verbatim — they are implementation-defined and treated as opaque metadata.

### Fields

| Field | Required | Description |
|---|---|---|
| `id` | Yes | UUID v4. Permanent. Never changes. The canonical identity in the graph. |
| `name` | Yes | Human-readable name. Vault-wide unique (case-insensitive). Mutable. The linking handle. Defaults to first line of content on creation. |
| `created` | Yes | ISO 8601 creation timestamp. |
| `modified` | Yes | ISO 8601 last modification timestamp. Updated on every content mutation. |

### Name Rules

- Names are **vault-wide unique, case-insensitive**. No two blocks may share a name that differs only by capitalization. `Notes` and `notes` are a collision. This ensures filenames are unambiguous on all platforms (Linux, macOS, Windows).
- Names **must not contain `[` or `]`**. These characters are reserved for inline reference syntax (`[[Name]]` and footer `[Name]: uuid:`). A conforming implementation rejects block creation or rename when the name contains either character.
- On creation, `name` defaults to the first line of the block's content, truncated to 120 characters.
- On collision, a numeric suffix is appended automatically: `Getting Started (2)`.
- Name and content are **decoupled after creation.** Editing content never changes `name`. Renaming never changes content. The name is a stable linking handle, not a content mirror.
- On rename, the implementation updates the filename on disk to match the new encoded name. This is an infrastructure concern — the domain returns the rename result, the adapter performs the filesystem operation.

### Markdown Content Rules

When `format` is `"markdown"`:

- Content is CommonMark compliant.
- **No heading syntax (h1–h6) inside block content.** Headings are block boundaries — encountering one during parsing ends the current block and begins a new one. A heading inside a stored block file is a parse error.
- Heading syntax inside fenced code blocks is permitted — it is content, not structure.
- Inline formatting (bold, italic, code spans, links) is permitted.

### Inline Block References

A block references another block by name using double-bracket syntax:

```markdown
See also [[Getting Started]] for more context.
```

Inline references use the human-readable `name`, never the UUID. References are live — they resolve at read time against the current heap. When a block is renamed, all inline references to it are updated automatically by the system (every occurrence of the reference syntax in content is updated; the implementation does not distinguish code blocks or other literal context — content is treated as a single string for propagation). When a referenced block is deleted, inline references are reverted to plain text (the `[[brackets]]` are removed, leaving only the name string).

### Footer Annotations

Every inline reference in a block's content must have a corresponding footer annotation mapping the name to the target UUID. Footer annotations are the bridge between human-readable content and the graph.

```markdown
<!--
id: a3f9b2c1-...
name: My Analysis
-->

See also [[Getting Started]] for context. This [[Key Insight]] elaborates further.

<!-- refs -->
[Getting Started]: uuid:b4e8d3f2-...
[Key Insight]: uuid:c7a1e9d4-...
```

Footer annotations are maintained by the system, not hand-written by the user. On rename, the system updates the annotation. On deletion of a target block, the annotation is removed and the inline reference is reverted to plain text. `block-graph.json` is authoritative — footer annotations are the human-readable, git-diffable record of the same edges.

---

## 3. Block Reference Graph (`block-graph.json`)

The primary knowledge structure. Directed edges between block UUIDs. This is the canonical graph — edges are stored by UUID, not by name, so they survive renames without modification.

The graph is live. It watches its members. When a block is renamed, the graph does not change — edges are UUID-based — but the system propagates the new name to all footer annotations and inline references in block content.

### Schema

```json
{
  "version": "0.1.0",
  "edges": [
    {
      "id": "uuid-v4",
      "source": "uuid-v4",
      "target": "uuid-v4"
    }
  ]
}
```

### Edge Fields

| Field | Type | Description |
|---|---|---|
| `id` | UUID v4 | Permanent edge identity. |
| `source` | UUID v4 | Source block. Must exist in heap. |
| `target` | UUID v4 | Target block. Must exist in heap. |

An edge means: this block references that block. The meaning of the relationship lives in the content, not in a system tag.

### Block Reference Rules

- Block → block only. Both source and target must be block UUIDs in the heap. Cross-vault references are not permitted.
- Freely cyclic. Cycles are valid and expected.
- Order is irrelevant. The edge list is unordered.
- Referential integrity: every UUID in `source` or `target` must exist in the heap. Dangling UUIDs are a validation error.
- Every `[[Name]]` inline reference in any block's content must have a corresponding edge in `block-graph.json`. The graph and footer annotations are always consistent.

---

## 4. Documents (`/documents`)

Documents are **optional views** over the block heap. They are second-class to the block graph — the graph is the knowledge base, documents are a presentation layer. For typical and casual users, documents are the natural way to read and navigate: a linear or hierarchical arrangement of blocks that renders as a familiar document tree.

A document does not own its blocks; the heap does. The same block may appear in multiple documents. Rearranging or deleting a document never affects the heap or the block graph. Internal links within a document are just block-graph edges between blocks that happen to appear in the same document — there is no separate document-internal link structure. `[[Block Name]]` in content resolves via the block graph regardless of which document view you're in.

Each document is a single JSON file in `/documents`, named by UUID: `<uuid>.json`.

### Document Identity

Every document has a **root block** — the block whose `name` is the document's title. The root block is the `h1` block: the first block in the document. `[[Document Title]]` in any block's content resolves to the document's root block UUID. Document-level linking is block-level linking — there is no separate document entity in the graph.

### Schema

```json
{
  "id": "uuid-v4",
  "root": "uuid-v4",
  "sections": [
    {
      "block": "uuid-v4",
      "subsections": [
        { "block": "uuid-v4" }
      ]
    },
    {
      "block": "uuid-v4",
      "subsections": []
    }
  ]
}
```

### Fields

| Field | Type | Description |
|---|---|---|
| `id` | UUID v4 | Permanent document identity. |
| `root` | UUID v4 | Root block UUID. Block's `name` is the document title. Renders as h1. |
| `sections` | array | Ordered top-level sections. Each section is a block rendering at h2. |
| `sections[].block` | UUID v4 | Section block UUID. Must exist in heap. |
| `sections[].subsections` | array | Ordered subsection blocks. Each renders at h3. Max one level deep. |

A document node is a reference to a block UUID plus its position in the hierarchy. No additional node properties are required — the block's own `name` and content supply what the view needs. Implementations may add optional per-node metadata (e.g. display title override for this document only) as long as the core schema remains valid.

### Document Properties

- **Two levels of intra-document hierarchy.** Root (h1) → sections (h2) → subsections (h3). Content requiring deeper hierarchy becomes a new document, with a `[[reference]]` edge from the subsection block to the new document's root block.
- **Non-exclusive membership.** A block UUID may appear in multiple documents. The heap owns the block.
- **Documents are flat.** Documents do not nest within documents. Relationships between documents are expressed as block-level reference edges between their respective root blocks.
- **Acyclic.** A block may not be both an ancestor and a descendant of itself within a document.

The document definition is the sole input to rendering. Walking root → sections → subsections in order produces the output .md document tree (see §7). No separate export format — the document model is the export model.

### Orphaned Blocks

A block with no edges in `block-graph.json` — no incoming and no outgoing references — is an orphan. Orphans are valid. The heap owns them. Conforming implementations surface orphans in a heap browser so they remain accessible and can be connected or discarded. A block not appearing in any document is not an orphan — documents are optional views and absence from them carries no meaning.

---

## 5. Mutation Standards

All state changes are commands. Queries never mutate state. Validation occurs before commitment. Failed commands are rejected with a descriptive error. Successful commands produce a complete, ordered set of writes that are applied atomically via the commit protocol (see §5a). The checksum and `previous_checksum` in the manifest are updated as the final step of every commit.

### Mutation gate (when mutations are permitted)

Before applying any mutation, a conforming implementation must enforce the following gate. The gate is evaluated per request (e.g. per command); implementations may skip the checksum check when they have just completed a successful commit and no other process could have modified the vault (implementation-defined).

1. **Checksums match.** Recompute the vault checksum using the algorithm in §1 and compare to `manifest.checksum`. If they match, there is no obstacle; the implementation may proceed to execute the command and apply its writes.

2. **Checksums mismatch.** Run full validation: verify all domain invariants (§6) and load-time rules (§6) hold for the current vault state. If validation reports **no** violations, there is no obstacle; the implementation may proceed. The implementation should update `manifest.checksum` to reflect the current state (e.g. when committing this or a subsequent mutation) so the vault is no longer drifted. If validation reports **one or more** violations, the implementation **must not** apply any mutation. Remediation is required: either the spec is extended to define acceptable repair for the reported condition, or human input (or an implementation-defined repair tool) must resolve the violations before any mutation is permitted.

3. **Remediation required.** When the gate blocks (validation failed after a checksum mismatch), the implementation must report the violation(s) and refuse the mutation until the vault state satisfies all invariants. The implementation may allow read-only access or a remediation workflow; it must not apply writes.

Compliance: an implementation that permits a mutation when the gate would block (e.g. checksum mismatch and validation reports violations) is non-conforming. An implementation that blocks mutation when checksums match, or when checksums mismatch but validation passes, is also non-conforming.

### Commands

#### Block Commands

| Command | Description | Validates |
|---|---|---|
| `AddBlock` | Add a new block file to `/blocks`. | UUID unique, name unique, metadata complete. |
| `RenameBlock` | Change a block's `name`. Propagates to all inline refs and footer annotations vault-wide. | Block exists, new name unique vault-wide. |
| `MutateBlockContent` | Update block content. Updates `modified` timestamp. | Block exists, content valid for declared format, no heading syntax outside fenced code. |
| `DeleteBlockSafe` | Delete block. Fails if incoming reference edges exist. | No incoming edges in `block-graph.json`. |
| `DeleteBlockCascade` | Delete block. Removes all incoming and outgoing edges. Reverts all inline `[[Name]]` references in other blocks to plain text. Removes corresponding footer annotations. Emits warning with counts. | Block exists. |

#### Document Commands

| Command | Description | Validates |
|---|---|---|
| `AddDocument` | Create a new document definition in `/documents`. | UUID unique, root block exists in heap. |
| `AppendSection` | Add a block as a section to a document. | Document exists, block exists in heap, depth limit respected, block not already a section ancestor. |
| `AppendSubsection` | Add a block as a subsection under a section. | Document exists, parent section exists, block exists in heap. |
| `RemoveSection` | Remove a section (and its subsections) from a document. | Document exists, section present. Does not delete blocks. |
| `ReorderSections` | Reorder a document's sections list. | Same block UUIDs, different order. |
| `DeleteDocument` | Delete document definition. | Document exists. Does not delete blocks or graph edges. |

#### Reference Commands

| Command | Description | Validates |
|---|---|---|
| `AddEdge` | Add an edge to `block-graph.json`. | Source and target exist in heap. |
| `RemoveEdge` | Remove an edge by edge UUID. | Edge exists. |

### Events

Every successful command emits a domain event. Events are not persisted in v0.

| Event | Payload |
|---|---|
| `BlockAdded` | block UUID, name |
| `BlockRenamed` | block UUID, old name, new name, refs_updated count |
| `BlockContentMutated` | block UUID |
| `BlockDeleted` | block UUID, edges_removed count, inline_refs_reverted count |
| `DocumentAdded` | document UUID, root block UUID |
| `SectionAppended` | document UUID, block UUID, depth |
| `SectionRemoved` | document UUID, block UUID |
| `SectionsReordered` | document UUID |
| `DocumentDeleted` | document UUID |
| `EdgeAdded` | edge UUID, source, target |
| `EdgeRemoved` | edge UUID |
| `VaultOpened` | vault UUID, checksum_status |
| `ChecksumMismatch` | expected, actual, drift_summary |

---

## 5a. Commit Protocol

Every successful command produces a complete, ordered list of vault writes. Applying those writes to persistent storage must be crash-safe: either all writes land and are reflected in the manifest, or the vault can be returned to a clean prior state. This section defines the normative commit protocol that achieves this guarantee.

### Vault Structure Addition

```
/portablenote/
  .journal          # Ephemeral. Present only during an in-flight commit.
```

The `.journal` file is a temporary artifact. It must not be present in a cleanly committed vault. Its presence on vault open is the indicator that a prior commit did not complete.

### Journal Format

The journal is a JSON file with the following schema:

```json
{
  "expected_checksum": "sha256:<hex>",
  "before_image": [
    { "kind": "Block", "data": { ... } },
    { "kind": "Edge", "data": { ... } },
    { "kind": "Document", "data": { ... } },
    { "kind": "Name", "name": "...", "id": "uuid-v4" }
  ],
  "writes": [
    { "kind": "WriteBlock", "data": { ... } },
    { "kind": "DeleteBlock", "id": "uuid-v4" },
    { "kind": "WriteEdge", "data": { ... } },
    { "kind": "RemoveEdge", "id": "uuid-v4" },
    { "kind": "WriteDocument", "data": { ... } },
    { "kind": "DeleteDocument", "id": "uuid-v4" },
    { "kind": "SetName", "name": "...", "id": "uuid-v4" },
    { "kind": "RemoveName", "name": "..." }
  ]
}
```

#### Journal Fields

| Field | Description |
|---|---|
| `expected_checksum` | The checksum the vault will have after all writes land. The "after" state. |
| `before_image` | Full prior state of every artifact that `writes` will modify or delete. Sufficient to fully undo the operation. |
| `writes` | The ordered list of writes to apply. Applied in sequence. |

#### Before-Image Entries

Each entry in `before_image` records the artifact's state immediately before the commit:

- An artifact that **existed before** the commit: `{ "kind": "<Kind>", "data": <full serialized artifact> }`
- An artifact **created by this commit** (no prior state): `{ "kind": "<Kind>", "id": "<uuid>", "data": null }` — undo means delete it

Kinds: `"Block"`, `"Edge"`, `"Document"`, `"Name"` (name entries use `"name"` and `"id"` fields; `"data": null` means undo = remove the name entry).

### Commit Ordering

A conforming implementation must apply writes in this exact order:

1. **Write `.journal`** — Atomically (temp file + rename on POSIX, or equivalent). The journal must be fully durable before any vault artifact is modified.
2. **Apply writes** — Apply every entry in `writes` in order to the vault artifacts (blocks, graph, names, documents).
3. **Write manifest** — Atomically write `manifest.json` with `checksum` set to `expected_checksum` and `previous_checksum` set to the current `manifest.checksum` (the pre-commit state). This is the **commit point**: once the manifest is written, the commit is complete.
4. **Delete `.journal`** — Clean up.

Step 3 is the commit point. If the process crashes after step 1 but before step 3, the journal is present on next open and recovery applies. If the process crashes after step 3, the journal may or may not be deleted — that is harmless, as recovery will detect Case A.

### Recovery Protocol

On vault open, a conforming implementation must execute the following recovery check before any other validation:

**If `.journal` is present:**

Compute the actual checksum from the current vault contents (using the canonical serialization algorithm in §1).

| Case | Condition | Action |
|---|---|---|
| **A — Writes landed, manifest lost** | actual == `expected_checksum` | Rewrite manifest: set `checksum` to `expected_checksum`, set `previous_checksum` to current `manifest.checksum`. Delete journal. Vault is clean. |
| **B — No writes landed** | actual == `manifest.checksum` | Delete journal. Vault is clean at prior state. |
| **C — Partial writes** | actual matches neither | Apply undo: restore every entry in `before_image` to disk, recompute checksum, verify it matches `manifest.checksum`. Rewrite manifest with `manifest.checksum` as `checksum` (no change to `previous_checksum`). Delete journal. Emit a warning. |

Case C undo is the mandated recovery strategy. Implementations must not silently accept a partial-write state.

After successful recovery, the vault is in a fully consistent state and proceeds with normal open-time validation.

**If `.journal` is absent:**

Compute checksum and compare to manifest. A mismatch without a journal indicates drift (e.g. external modification). The mutation gate (§5) applies: run full validation; if it passes, mutation is permitted; if it fails, remediation is required.

---

## 6. Validation Invariants

### Domain Invariants

These invariants must hold after every mutation. Conforming implementations enforce all of them.

1. Every UUID in `block-graph.json` source or target fields exists in the heap.
2. Every block UUID in a document's `root`, `sections`, or `subsections` fields exists in the heap.
3. No block is its own ancestor within a document (acyclicity).
4. `block-graph.json` contains only block → block edges.
5. Every `[[Name]]` inline reference in any block's content has a corresponding footer annotation and a corresponding edge in `block-graph.json`.
6. Every footer annotation maps to a name that resolves to an existing block in the heap.
7. Block names are vault-wide unique (case-insensitive). No two blocks share a `name` that differs only by capitalization.
8. No block content contains heading syntax (h1–h6) outside fenced code blocks when format is `"markdown"`.

### Load-Time Rules

These rules are enforced when a vault is opened, not after every mutation. They concern on-disk representation rather than domain state.

9. Every block filename matches `encode(metadata.name) + extension`. The metadata `name` is authoritative; mismatched filenames are corrected on open (not treated as a validation failure).
10. The manifest checksum reflects the current state of all source artifacts. Mismatch triggers the mutation gate — see §5 Mutation gate.

---

## 7. Rendered Output

Each composition produces a rendered Markdown document tree at the vault root under `/<composition-name>/`. This replaces the concept of a separate "export" operation — rendering is continuous, not a manual step.

### Output Layout

```
/<composition-name>/
  <document-title>.md
  <document-title>/
    <section-title>.md
```

### Rendering Rules

- A block's `name` is rendered as a heading. Heading level is determined by the block's position in the document definition: root = h1, section = h2, subsection = h3. The block's content contains no heading syntax — the heading is emitted by the renderer.
- Block content is rendered as-is in document order.
- Inline block references render as wikilinks using the target block's current `name`: `[[Block Name]]`.
- Block-graph edges are not represented in rendered output. They are a source artifact only.
- Rendering is fully reactive. Every domain event that mutates source artifacts triggers an output rebuild for affected compositions. The rendered tree is never edited directly — it is always derived from source artifacts.
- Rendered output may be committed to git for sharing/portability, or gitignored. That is the user's choice.

---

## 8. Import Standard (Markdown Vaults)

Conforming implementations should support import from existing Markdown vaults (Obsidian, Logseq export, plain `.md` directories).

### Heading → Block Boundary Rules

Every heading encountered during import ends the current block and begins a new one. The heading text becomes the new block's `name`. Block content is everything between that heading and the next heading (or end of file).

| Heading | Becomes |
|---|---|
| `h1` | Root block. Block `name` = heading text. Document `root` = this block's UUID. Renders as h1. |
| `h2` | Section block. Added to document `sections`. Renders as h2. |
| `h3` | Subsection block. Added to parent section's `subsections`. Renders as h3. |
| `h4+` | New document is created. A reference edge is added from the h3 subsection block to the new document's root block. Deep hierarchy becomes graph structure. |

On name collision during import, a numeric suffix is appended automatically.

### Wikilink Conversion

`[[Page Name]]` wikilinks are resolved to block names where possible. A resolved wikilink becomes `[[Block Name]]` with a footer annotation mapping the name to the target UUID and a corresponding edge added to `block-graph.json`. Unresolvable wikilinks are preserved as plain text with a warning emitted.

### Metadata Mapping

Existing metadata fields not recognized by the spec are preserved in the block's metadata comment verbatim.

---

## 9. Content Format Adapter

The content format adapter is a port. The Markdown adapter is the v0 reference implementation. Future adapters (RTF, HTML, Portable Text) implement the same behavioral contract.

### Required Capabilities

A conforming content format adapter must provide:

- **Parse** — Read a block file and extract structured content (metadata, body, footer annotations).
- **Serialize** — Write structured content back to a block file in the declared format.
- **Validate** — Check a raw content string against the format's rules (e.g. no headings outside fenced code for Markdown).
- **Extract inline references** — Return all inline block reference names (`[[Name]]`) from a content string.

### Format Declaration

Format is declared once per vault in the manifest. All blocks in a vault use the same format. Format migration between vaults requires re-serialization through the adapter. Mixing formats within a vault is not permitted in v0.

---

## Appendix A: Graph Layout Convention (Non-Normative)

Graph visualization metadata is **not a spec artifact**. It is excluded from the checksum, excluded from validation, and not required for compliance. A conforming implementation may ignore it entirely.

This convention exists so that implementations that support spatial graph views can share layout data portably. The file travels with the vault but carries no contractual weight.

### File

```
/portablenote/graph-layout.json
```

### Schema

```json
{
  "nodes": {
    "<block-uuid>": {
      "x": 0,
      "y": 0,
      "size": 0,
      "weight": 0
    }
  },
  "edges": {
    "<edge-uuid>": {
      "tension": 0
    }
  }
}
```

All fields within node and edge entries are optional. An empty entry (`{}`) is valid and means "defer to implementation defaults." Absent UUIDs mean the same — no layout opinion for that node or edge.

### Scale

All values use a single unified scale: **`-100` to `+100`**, where `0` is the implementation's default.

| Field | Range | Meaning of `0` | Meaning of `-100` / `+100` |
|---|---|---|---|
| `x` | -100 to +100 | Horizontal center | Far left / far right |
| `y` | -100 to +100 | Vertical center | Top / bottom |
| `size` | -100 to +100 | Declared node size | Minimum / maximum |
| `weight` | -100 to +100 | Declared node weight | Minimum / maximum |
| `tension` | -100 to +100 | Declared pull strength | Longer / shorter |

### Keying

Nodes are keyed by block UUID. Edges are keyed by edge UUID. This ensures renames do not require layout file updates.

### Stale Entries

Entries referencing UUIDs that no longer exist in the vault are harmless and may be pruned lazily by the implementation. No validation error is raised.

### Compliance Boundary

A conforming implementation **must not** include `graph-layout.json` in the checksum computation. A compliance test should verify: modifying, adding, or removing layout entries does not change the vault checksum.

---

## Appendix B: Open Questions for v0.2+

- Compliance suite scope: minimal (valid/invalid fixtures + a few mutation scenarios) for v0, or fuller scenario coverage? Harness format (CLI, library, language-agnostic scenario files)?
- Compliance certification: informal for v0 (suite exists, implementations run it); registry or badge model for v1+ if the ecosystem grows.
- Template system: first-class spec entry or implementation-defined convention?

---

*PortableNote Specification v0.1.0-draft. Subject to revision. Portability equals ownership.*
