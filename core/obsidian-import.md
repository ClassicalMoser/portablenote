# Obsidian Import Strategy

**Status:** Design document. Not yet implemented.

This describes how a conforming PortableNote implementation should import an Obsidian vault. The general import standard is defined in [portablenote-spec.md, Section 8](../spec/portablenote-spec.md#8-import-standard-markdown-vaults). This document extends that standard with Obsidian-specific design decisions for our implementation.

---

## Core Insight

An Obsidian document is already a composition of blocks separated by headings. We just don't have the graph yet. The import process deconstructs Obsidian's flat files into PortableNote's block graph, then offers documents as recomposed views that preserve the original navigability while exposing richer structure.

## Import Pipeline

### 1. Deconstruct documents into blocks

Each Obsidian `.md` file is split at heading boundaries per the spec's import rules:

- **h1** becomes the root block of a new document. Its text becomes the block name.
- **h2** becomes a section block, appended to the document's `sections`.
- **h3** becomes a subsection block under its parent h2 section.
- **h4+** triggers creation of a new document, with a reference edge from the h3 block back to the new document's root block. Deep hierarchy becomes graph structure rather than deeply nested content.

Content between headings becomes the block body. The first block inherits the document role: if the file starts without a heading, the filename (sans extension) is used as the root block name.

### 2. Resolve wikilinks into block-reference links

Obsidian's `[[Page Name]]` links are converted to PortableNote's canonical block-reference format:

- For each `[[Page Name]]` wikilink, find the block whose name matches (the root block of the page with that name).
- Replace with `[Block Name](block:target-uuid)` and add a corresponding edge to `block-graph.json`.
- Wikilinks to headings (`[[Page Name#Heading]]`) resolve to the block created from that heading, since headings become block boundaries.
- Unresolvable wikilinks are preserved as plain text with a warning.

This is where the model shines: links to *headings* become links to *blocks*, because headings are block boundaries. What was a deep-link into a monolithic page becomes a first-class graph connection.

### 3. Reconstruct documents as views

After deconstruction, the original Obsidian file structure is recoverable as a PortableNote document:

- One document per original `.md` file.
- The root block is the h1 (or filename-derived) block.
- Sections and subsections mirror the original heading hierarchy.
- The rendered output at the vault root reproduces the original navigable structure.

The user sees the same content, organized the same way. But now each section is an independent block that can be referenced, recomposed into other documents, and connected in the graph.

### 4. Preserve unrecognized metadata

Obsidian frontmatter fields not recognized by PortableNote (tags, aliases, cssclass, etc.) are preserved verbatim in the block's HTML comment metadata. No data is discarded during import.

## What This Enables

- **Obsidian's page links become block-reference links.** Every `[[Page Name]]` becomes `[Block Name](block:uuid)` and a graph edge. Every `[[Page Name#Heading]]` becomes a link to a specific block rather than an anchor in a monolith.
- **Heading sections become composable blocks.** A section written under one document can appear in another without copy-paste. The graph tracks the relationship.
- **Documents remain navigable.** The reconstructed documents preserve the original reading order. Non-technical users see familiar structure. Power users see the graph.
- **Richer editing.** Blocks can be reordered, moved between documents, or split further. The graph adapts. The original Obsidian structure is a starting point, not a prison.

## Open Questions

- **Folder structure mapping.** Obsidian uses folders for organization. PortableNote doesn't have a folder concept (documents are flat compositions). Folder hierarchy could map to document grouping metadata or be flattened. Needs design.
- **Obsidian plugins and custom syntax.** Dataview queries, Templater blocks, and other plugin syntax should be preserved as raw content but won't be functional. Warning on import.
- **Embedded files.** Obsidian's `![[image.png]]` embed syntax needs a convention for binary asset references. Out of scope for v0 but should not block import (preserve as plain text).
- **One-way import.** This is import-only. No one goes from an open spec back to a closed one.
