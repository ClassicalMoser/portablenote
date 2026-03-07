# Correctness & Spec Compliance Audit

**Date:** 2025-03  
**Focus:** Correctness over completeness; spec–implementation gaps.

---

## 1. Critical correctness issues

### 1.1 Case-insensitive name uniqueness not enforced at command time — DONE

**Spec (§2 Name Rules):** “Names are vault-wide unique, case-insensitive. No two blocks may share a name that differs only by capitalization.”

**Implementation (updated):** `NameIndex` now has `resolve_ignore_case(name) -> Option<(String, Uuid)>`. `add_block` and `rename_block` use it before create/rename; names are stored exactly, comparison normalizes on check. Case-insensitive duplicates are rejected at command time.

**Compliance:** `add-block-duplicate-name-case-insensitive.json` and `rename-block-duplicate-name-case-insensitive.json` added; both expect rejected.

---

### 1.2 Recovery Case C: strategy and spec

**Spec (§5a Recovery):** For Case C (partial writes), the spec currently mandates undo: restore from `before_image`, verify checksum, rewrite manifest, delete journal, emit warning.

**Implementation:** We apply undo and delete the journal. We do not recompute/verify after undo, rewrite manifest, or emit a warning.

**Design note:** Undo is not the only option. We have the journal (writes + before_image + expected_checksum), so we could **reattempt** the commit (re-apply writes, then manifest, then delete journal), possibly with backoff; if reattempt fails, then fall back to **undo with warning**. The spec guarantees **directory state** (vault is consistent after recovery) more than it guarantees a specific behavior (undo vs reattempt). So recovery strategy can be implementation-defined as long as the resulting state is correct. Our current "undo only" is one valid choice; adding reattempt-with-backoff then undo-with-warning would be a reasonable extension. Spec could be relaxed to allow implementation-defined recovery (reattempt and/or undo) as long as the final state matches manifest.

---

## 2. Spec–implementation gaps (interop / load-time)

### 2.1 Checksum normalization (§1)

**Spec:** Canonical checksum uses: UUIDs lowercase hyphenated; name and string fields UTF-8, NFC-normalized; line endings in content LF only (`\r\n` normalized to `\n` on write).

**Implementation:**
- **UUIDs:** Rust `Uuid` `Display` is already lowercase hyphenated. OK.
- **NFC:** We do not NFC-normalize block names or content before hashing. Different implementations may produce different checksums for equivalent logical content.
- **Line endings:** We do not normalize `\r\n` → `\n` when writing block content. Stored content is hashed as-is; spec says `\r\n` in stored content is a format violation and should be normalized on write.

**Missing tests:** The suite does not assert that checksum is stable under NFC normalization or under LF normalization of content. We need **critical tests** that verify: (1) same logical content in NFC form produces the same checksum; (2) content with `\r\n` normalized to `\n` produces the expected checksum (or that we normalize on write and hashing is consistent). Without these, normalization bugs or interop drift are easy to miss.

**Fix:** (1) Add compliance or unit tests for NFC and LF normalization vs checksum. (2) Implement normalization (NFC and LF on write / in checksum) so behavior matches spec.

---

### 2.2 Filename vs metadata: when to correct (§6 Load-Time Rules)

**Spec (Rule 9) as written:** "Mismatched filenames are corrected **on open**."

**Assessment:** That is the wrong place. Metadata is authoritative; the implementation should **fix the filename on mutation** (when we write a block, we always write to `encode(block.name) + extension`). So any prior mismatch is corrected the next time that block is saved. Correcting on open is unnecessary and pushes logic into the load path. **Checksum** should guard against tampering: if someone renames a file on disk without changing metadata, the next checksum run will show drift and the mutation gate (or load-time check) will surface it. So: fix on **mutation** (current behavior is correct); use **checksum** to guard tampering; **spec should be updated** to say "mismatched filenames are corrected on next write of that block (metadata is authoritative)" rather than "corrected on open."

---

### 2.3 Journal before_image format for Name (§5a)

**Spec:** Name entries in `before_image`: `{ "kind": "Name", "name": "...", "id": "uuid-v4" }` at top level (no `data` wrapper).

**Implementation:** `BeforeImageEntry::Name { name, id }` with `#[serde(tag = "kind", content = "data")]` serializes as `{"kind": "Name", "data": {"name": "...", "id": ...}}`.

**Effect:** Our journal is self-consistent and recovery works. Another implementation that reads our journal might expect top-level `name` and `id` and fail or misparse.

**Fix:** Custom serialize/deserialize for the Name variant so the on-disk shape matches the spec (top-level `kind`, `name`, `id`).

---

## 3. Softer gaps (validation vs. command-time reject)

### 3.1 AppendSection / AppendSubsection: "block not already a section ancestor" — DONE

**Spec (§5 AppendSection):** Validates "block not already a section ancestor."

**Implementation (updated):** `append_section` use case checks `block_already_in_document` (root or any section/subsection) before calling domain; returns `DomainError::BlockAlreadyInDocument(block_id)` when the block is already in the hierarchy.

**Compliance:** `append-section.json` expects rejected when block is already subsection; `append-section-duplicate-root.json` expects rejected when block is root.

---

### 3.2 DeleteBlockCascade and documents

**Spec (§5 DeleteBlockCascade):** “Delete block. Removes all incoming and outgoing edges. Reverts all inline `[[Name]]` references… Removes corresponding footer annotations. Emits warning with counts.” It does not explicitly say documents are updated.

**Implementation:** We do not remove the deleted block from any document’s root/sections/subsections. Documents can end up with dangling block UUIDs.

**Effect:** `check_document_block_refs` then reports violations; mutation gate blocks until the user fixes or removes the document. So behavior is “remediate,” not automatic document update.

**Assessment:** Arguably a design choice. If the spec is read as “documents are views; we don’t auto-edit them,” leaving dangling refs and requiring remediation is consistent. If the spec is read as “cascade should leave no dangling refs,” we’d need to emit document updates (e.g. remove section/subsection or drop doc) when the root or a section block is deleted. Worth clarifying in the spec and/or adding a note in ARCHITECTURE.

---

## 4. Verified correct

- **Commit order (§5a):** Write journal → apply writes → write manifest → delete journal. Implemented in CLI `commit_with_journal`.
- **Recovery A/B:** Case A (rewrite manifest, delete journal) and Case B (delete journal) implemented and tested.
- **Mutation gate (§5):** Checksum match → allow; mismatch → full validation; violations → block. Gate and `RemediationRequired` behavior match spec.
- **Reserved characters in names:** `[` and `]` rejected in domain and in invariants.
- **Invariants (§6):** All eight domain invariants and the documented load-time rule (checksum/mutation gate) are implemented: edge endpoints, document refs, acyclicity (no duplicate block in hierarchy), inline ref + footer + edge consistency, footer targets, name uniqueness (case-insensitive in validation), no headings outside fenced code.
- **Document acyclicity:** “No block is its own ancestor” implemented as “no block appears more than once in root/sections/subsections”; matches the spec’s structure.
- **delete_block_safe:** Uses `incoming(block_id)` to reject; only removes edges where `source == block_id` in practice (because incoming is empty when we proceed). Behavior correct.
- **Checksum structure:** Blocks, edges, documents order and format match spec; timestamps excluded; `names.json` excluded.

---

## 5. Compliance suite gaps (missing cases)

The suite has **26 mutation scenarios** (up from 20 after Phase 1+2) and several **invalid fixtures** (load-time validation). Remaining gaps:

### Mutation scenarios (command → expected result + assertions)

| Gap | Scenario idea | Status |
|-----|----------------|--------|
| ~~Case-insensitive name~~ | `add-block-duplicate-name-case-insensitive.json` | **Done** (Phase 1) |
| ~~Case-insensitive rename~~ | `rename-block-duplicate-name-case-insensitive.json` | **Done** (Phase 1) |
| ~~Reserved characters~~ | `add-block-reserved-bracket.json`, `rename-block-reserved-bracket.json` | **Done** (Phase 2) |
| ~~Encoding round-trip~~ | `add-block-special-chars.json` | **Done** (Phase 2) |
| **Long name truncation** | AddBlock with name >200 bytes (after encoding) → success | Open |
| ~~AppendSection duplicate~~ | `append-section.json`, `append-section-duplicate-root.json` | **Done** (Phase 2) |
| ~~Manifest after mutation~~ | CLI integration test only (documented decision) | **Done** (Phase 2) |

### Checksum / normalization (unit or compliance)

| Gap | What to test |
|-----|--------------|
| **NFC** | Same logical content in NFC form produces same checksum; or names/content normalized to NFC before checksum. |
| **LF** | Content with `\r\n` normalized to `\n` (on write or in checksum) produces expected/stable checksum. |

### Invalid fixtures (load-time validation)

Existing: `duplicate-name`, `bad-checksum`, `dangling-uuid`, `duplicate-uuid`, `heading-in-block`, `missing-frontmatter`. Potentially missing or worth adding:

- **Reserved in name:** vault with a block whose name contains `[` or `]` → expected violation.
- **NFC / LF** (if we add normalization): fixtures that are invalid until normalized, or that document expected behavior.

### Recovery / journal

- **Case A** (writes landed, manifest lost): scenario or integration test that leaves journal + applied writes, no manifest write, then "open" and assert manifest restored and journal deleted.
- **Case C** (partial writes): scenario that leaves journal + partial writes, then "open" and assert undo applied and vault consistent (and optionally warning).
- **Journal format**: round-trip serialization of journal (including Name entries) if we care about interop.

### Summary

After Phase 1+2, the mutation scenario gaps are mostly closed. Remaining: **long name truncation** (optional), **normalization tests** for checksum (NFC + LF), **invalid fixtures** (reserved-in-name), and **recovery/journal** integration tests (Case A, C, journal format). The harness and assertion types are in good shape.

---

## 6. Summary

| Category              | Count | Status |
|-----------------------|-------|--------|
| Critical correctness | 1     | **Done** — case-insensitive name at command time (1.1) |
| Recovery Case C      | —     | Open — spec could allow reattempt-then-undo; implementation-defined strategy |
| Spec/interop gaps    | 3     | Open — NFC + LF normalization (+ tests); spec fix for filename (fix on mutation); journal Name format |
| Softer gaps          | 2     | 1 done (AppendSection ancestor 3.1); 1 open (cascade vs. documents 3.2) |

**Next:** Phase 3 (checksum normalization, journal format, spec Rule 9 edit), then Phase 4 (recovery hardening, cascade decision).

---

## 7. Plan (prioritized)

### Phase 1 — Critical correctness (block non-conforming state) — DONE

1. **Case-insensitive name at command time (1.1)** — Implemented.  
   - `NameIndex::resolve_ignore_case(name) -> Option<(String, Uuid)>` added; implemented in `InMemoryNameIndex` and `FsNameIndex`.  
   - `add_block` and `rename_block` use it; unit tests and compliance scenarios added.

### Phase 2 — Compliance coverage (lock in behavior) — DONE

2. **Reserved characters in names** — Implemented.  
   - `add-block-reserved-bracket.json`, `rename-block-reserved-bracket.json` (expect rejected). Domain already rejected; no code change.

3. **Encoding and long names** — Implemented.  
   - `add-block-special-chars.json`: AddBlock name `"Notes: Part 1"` → success + `block_name_is` assertion.

4. **AppendSection ancestor (3.1)** — Implemented.  
   - `DomainError::BlockAlreadyInDocument(Uuid)`; `append_section` checks before calling domain.  
   - `append-section.json` (block already subsection → rejected), `append-section-duplicate-root.json` (block is root → rejected).

5. **Manifest after mutation** — Documented  
   - **Decision:** Keep as CLI integration test only (`manifest_checksum_updated_after_add_block`). Adding a compliance assertion type would require the harness to compute checksums and pull domain logic into the test runner. The CLI test covers the commit protocol end-to-end.  
   - No new compliance scenario added.

### Phase 3 — Spec alignment and interop

6. **Checksum normalization (2.1)**  
   - Add unit tests: NFC-normalized name/content → same checksum; content with `\r\n` → LF normalization → stable checksum.  
   - Implement: normalize to NFC and LF (on write and/or in `checksum::compute`) per §1.

7. **Journal Name format (2.3)**  
   - Custom serialize/deserialize for `BeforeImageEntry::Name` so JSON has top-level `kind`, `name`, `id` (no `data` wrapper).  
   - Add round-trip test for journal (including Name).

8. **Spec change: Rule 9 (2.2)**  
   - Propose spec edit: “Mismatched filenames are corrected on **next write** of that block (metadata is authoritative). Checksum guards tampering.”  
   - Remove or reword “corrected on open.”

### Phase 4 — Recovery and optional hardening

9. **Recovery Case A / C**  
   - Add integration test(s): leave journal + state (Case A: writes applied, no manifest; Case C: partial writes), open vault, assert recovery outcome and journal deleted.  
   - Optional: implement reattempt-with-backoff then undo-with-warning; document as implementation-defined.

10. **Cascade and documents (3.2)**  
    - Decide: remediate-only vs auto-update documents when root/section is deleted.  
    - Clarify in spec and/or ARCHITECTURE; if auto-update, implement and add scenario.

### Phase 5 — Cleanup and docs

11. **Invalid fixtures and suite docs**  
    - Add any missing invalid fixtures (e.g. reserved-in-name) noted in §5.  
    - Update spec/compliance README or audit with “what the suite covers” and how to add scenarios.

**Dependencies:** Phase 1 first (correctness). Phase 2 can be parallelized after 1. Phase 3 (normalization, journal format) feeds interop. Phase 4 is optional hardening. Phase 5 is ongoing.
