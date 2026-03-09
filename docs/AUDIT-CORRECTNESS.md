# Correctness & Spec Compliance Audit

**Date:** 2025-03  
**Focus:** Correctness over completeness; spec–implementation gaps.

---

## Does the spec accurately reflect the code?

**Short answer:** Mostly. The spec matches the core and infra on manifest, checksum, mutation gate, commit protocol, recovery, commands, and invariants. It **does not** match in one area (commit model); name and long-filename spec text has been corrected to match the implementation. **Always sending `expected_checksum: None` is a serious violation**; spec/compliance tests should catch it (see below). Details: (1) **commit model and rebase** — spec describes base + pending, fast/slow path, rebase on non-overlap; the implementation has none of that. (2) **Name rules** — spec now matches: name required on creation, reject on conflict (no auto-suffix). (3) **Long filenames** — spec says truncation is “disambiguated with a numeric suffix (2)”; the implementation truncates to 200 bytes at a character boundary but does not add a suffix or disambiguate.

### Where the spec matches the implementation


| Spec                                                                                                                           | Implementation                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| Manifest schema, checksum + previous_checksum                                                                                  | `Manifest`, `checksum::compute`, `is_drifted`; manifest write on commit            |
| Checksum canonical order (blocks → edges → documents), NFC/LF normalization                                                    | `domain/checksum.rs` matches §1                                                    |
| names.json excluded from checksum; updated on Add/Rename/Delete                                                                | Journal and commit path; names not in checksum                                     |
| Mutation gate: checksum match → allow; mismatch → full validation; violations → block; optional expected_checksum → StaleState | `gate::mutation_gate`, `MutationGate` port; infra builds vault and calls gate      |
| Commit protocol §5a: journal → apply writes → manifest → delete journal                                                        | `commit_with_journal` in CLI                                                       |
| Recovery Cases A/B/C, journal format, before_image (including Name entries)                                                    | `journal::recovery_case`, `undo_writes_from_journal`; Case C error if skipped > 0  |
| Block metadata (id, name, created, modified) in HTML comment; no heading in content                                            | `domain/format` parse/serialize; `blocks::create` / `apply_content` reject heading |
| Block filename: percent-encode per spec; metadata authoritative; fix on next write                                             | `infra/fs/encoding.rs`; block store uses encode; Rule 9 in spec                    |
| Name rules: no `[` `]` or `%`; case-insensitive uniqueness                                                                     | Domain + NameIndex `resolve_ignore_case`; add/rename reject                        |
| Commands (AddBlock, RenameBlock, DeleteBlockSafe/Cascade, etc.) and validations                                                | Use cases in `application/use_cases/`; domain commands and errors                  |
| §6 Domain invariants (edges, documents, acyclicity, block refs, name uniqueness, no heading)                                   | `invariants::validate_vault` covers all eight                                      |
| Rename propagates to inline refs; delete reverts refs; cascade updates documents                                               | `blocks::propagate_rename`, `revert_refs`; `delete_block_cascade` use case         |


### Where the spec does not match the implementation


| Spec says                                                                                                                                                                                                   | Code does                                                                                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **§5 Commit model and rebase:** Client keeps base + pending diff; fast path (checksum match → commit); slow path (diff, non-overlapping → rebase then atomic commit; overlapping → reject).                 | No base or pending diff. No diff/rebase/overlap logic. **Gate always called with `None`** — serious violation. Atomic commit only. See §2.4.                                             |
| **§2 Name rules:** *(spec corrected: name required, reject on conflict; no longer a mismatch)* “On creation, `name` defaults to the first line of the block's content, truncated to 120 characters.” “On collision, a numeric suffix is appended automatically: `Getting Started (2)`.” | Caller supplies `name` to `add_block`. No default from first line, no 120 truncation, no automatic suffix; name conflict returns `NameConflict` (reject). |
| **§2 Filename length:** “Names that exceed this after encoding are truncated and disambiguated with a numeric suffix: `Very Long Name That Exceeds... (2).md`.”                                             | `encode_block_filename` truncates to 200 bytes at a character boundary; no suffix and no disambiguation when two names truncate to the same string.       |


**Spec corrections made (no longer mismatches):** (1) Name rules — spec now says name is **required on creation** (caller supplies it) and **reject on conflict** (no auto-suffix). Import collision handling is out of scope. (2) Long filenames — spec now says truncation at 200 bytes; disambiguation may be defined in a future revision.

**Commit model: serious violation and compliance gap.** Always passing `expected_checksum: None` to the mutation gate when committing means the implementation never performs OCC and never triggers the slow path (rebase or reject). That violates §5. **Spec/compliance tests should catch this:** e.g. a scenario that asserts the commit path uses the vault's current manifest checksum as `expected_checksum` when invoking the gate, or a scenario that expects `StaleState` when the client's base does not match the remote manifest. Implementing the full commit model (base + pending, pass base to gate, rebase on non-overlap) will require a deep-dive; adding a compliance test that fails when the implementation always sends `None` is a first step.

---

## 1. Critical correctness issues

### 1.1 Case-insensitive name uniqueness not enforced at command time — DONE

**Spec (§2 Name Rules):** “Names are vault-wide unique, case-insensitive. No two blocks may share a name that differs only by capitalization.”

**Implementation (updated):** `NameIndex` now has `resolve_ignore_case(name) -> Option<(String, Uuid)>`. `add_block` and `rename_block` use it before create/rename; names are stored exactly, comparison normalizes on check. Case-insensitive duplicates are rejected at command time.

**Compliance:** `add-block-duplicate-name-case-insensitive.json` and `rename-block-duplicate-name-case-insensitive.json` added; both expect rejected.

---

### 1.2 Recovery Case C: strategy and spec

**Spec (§5a Recovery):** For Case C (partial writes), the spec currently mandates undo: restore from `before_image`, verify checksum, rewrite manifest, delete journal, emit warning.

**Implementation (updated):** We apply undo; if any journal (entry, write) pairs are skipped (corrupt/mismatched), we return an error. We then rebuild vault from disk, verify computed checksum equals pre-crash manifest checksum, rewrite manifest via `commit_manifest`, and delete the journal. `undo_writes_from_journal` now returns `UndoOutcome { writes, skipped }`; when `skipped > 0` recovery fails with an error instead of silently applying a partial undo.

**Design note:** Undo is not the only option. We have the journal (writes + before_image + expected_checksum), so we could **reattempt** the commit (re-apply writes, then manifest, then delete journal), possibly with backoff; if reattempt fails, then fall back to **undo with warning**. The spec guarantees **directory state** (vault is consistent after recovery) more than it guarantees a specific behavior (undo vs reattempt). So recovery strategy can be implementation-defined as long as the resulting state is correct. Our current "undo only" is one valid choice; adding reattempt-with-backoff then undo-with-warning would be a reasonable extension. Spec could be relaxed to allow implementation-defined recovery (reattempt and/or undo) as long as the final state matches manifest.

---

## 2. Spec–implementation gaps (interop / load-time)

### 2.1 Checksum normalization (§1) — DONE

**Spec:** Canonical checksum uses: UUIDs lowercase hyphenated; name and string fields UTF-8, NFC-normalized; line endings in content LF only (`\r\n` normalized to `\n` on write).

**Implementation (updated):**

- **UUIDs:** Rust `Uuid` `Display` is already lowercase hyphenated. OK.
- **NFC:** `checksum::compute` NFC-normalizes block name and content before hashing. `format::serialize_block_file` NFC-normalizes name and LF-normalizes content on write; parse defensively normalizes on read. `add_block` and `rename_block` NFC-normalize input names.
- **Line endings:** Content is LF-normalized in checksum and on write; CRLF/LF produce the same checksum.

**Tests:** Unit tests in `checksum` and `format` for NFC/LF stability; compliance covered.

---

### 2.2 Filename vs metadata: when to correct (§6 Load-Time Rules) — DONE

**Spec (Rule 9) updated:** Mismatched filenames are corrected **on the next write** of that block (not on open). Metadata is authoritative; checksum guards tampering. Spec text revised accordingly.

---

### 2.3 Journal before_image format for Name (§5a) — DONE

**Spec:** Name entries in `before_image`: `{ "kind": "Name", "name": "...", "id": "uuid-v4" }` at top level (no `data` wrapper).

**Implementation (updated):** `BeforeImageEntry` uses `#[serde(tag = "kind")]` with struct variants; Name serializes with flat `name`/`id` at top level. Round-trip tests added for journal (including Name entries).

---

### 2.4 Commit model and rebase (§5) — GAP

**Spec (§5 Commit model and rebase):** Client maintains **current base** (vault checksum last read/committed) and **pending diff** (writes to apply). Fast path: if `manifest.checksum` equals client base → apply pending writes via commit protocol. Slow path: if checksum differs → **diff** client base vs current remote; **non-overlapping** → **rebase** (recompute write set on top of current remote), then apply via atomic commit; **overlapping** → reject and surface for manual reconciliation.

**Implementation (current):**


| Spec requirement                                         | Code                                                                                                                                                                                    |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Client base + pending diff in memory                     | **No.** CLI does not track a “base” checksum or hold a pending diff across operations. Each command loads vault from disk, runs use case, gets writes, commits.                         |
| Pass base checksum into gate (fast path check)           | **No.** `VaultSession::require_gate()` always calls `gate.allow_mutation(None)`. OCC is only exercised in test (`mutation_gate_returns_stale_state_when_expected_checksum_mismatches`). |
| On checksum mismatch: diff base vs remote, overlap check | **No.** No logic to compare client base to current remote or to classify overlapping vs non-overlapping mutations.                                                                      |
| Rebase (recompute writes on top of current remote)       | **No.** No rebase step.                                                                                                                                                                 |
| Reject + surface on overlap                              | **Partial.** `StaleState` exists and could be surfaced if the CLI ever passed `Some(expected_checksum)`; no overlap-specific handling.                                                  |
| Atomic commit protocol after apply                       | **Yes.** `commit_with_journal` uses journal → apply writes → write manifest → delete journal.                                                                                           |


**Summary:** Atomic commit (§5a) is implemented. The **commit model and rebase** (base + pending, fast/slow path, diff/rebase/reject) is **not** implemented. Acceptable for single-writer / local-only usage; required for multi-client or sync scenarios that must conform to §5.

---

## 3. Softer gaps (validation vs. command-time reject)

### 3.1 AppendSection / AppendSubsection: "block not already a section ancestor" — DONE

**Spec (§5 AppendSection):** Validates "block not already a section ancestor."

**Implementation (updated):** `append_section` use case checks `block_already_in_document` (root or any section/subsection) before calling domain; returns `DomainError::BlockAlreadyInDocument(block_id)` when the block is already in the hierarchy.

**Compliance:** `append-section.json` expects rejected when block is already subsection; `append-section-duplicate-root.json` expects rejected when block is root.

---

### 3.2 DeleteBlockCascade and documents — DONE

**Spec (§5 DeleteBlockCascade) updated:** Cascade removes the block from every document that references it: removes the section or subsection; if the block is a document root, deletes that document. Spec and ARCHITECTURE updated.

**Implementation (updated):** `delete_block_cascade` takes `DocumentStore`, lists documents, and uses domain helpers `remove_block_from_document` / `remove_subsection` to emit `WriteDocument` or `DeleteDocument` writes so no dangling refs remain.

---

## 4. Verified correct

- **Commit order (§5a):** Write journal → apply writes → write manifest → delete journal. Implemented in CLI `commit_with_journal`.
- **Recovery A/B:** Case A (rewrite manifest, delete journal) and Case B (delete journal) implemented and tested.
- **Mutation gate (§5):** Checksum match → allow; mismatch → full validation; violations → block. Gate and `RemediationRequired` behavior match spec.
- **Optimistic concurrency (replacing base_version):** The manifest’s checksum chain (`checksum`, `previous_checksum`) is the state identity. `MutationGate::allow_mutation(expected_checksum)` accepts an optional `expected_checksum`; when set, the gate allows only if `vault.manifest.checksum == expected_checksum`, otherwise returns `StaleState`. Clients send the `checksum` they last read (after a commit that value becomes `previous_checksum`). No separate “base version” field on commands; the checksum serves that role. CLI still calls with `None`; core and integration tests cover `StaleState` when a stale checksum is passed.
- **Reserved characters in names:** `[` and `]` rejected in domain and in invariants.
- **Invariants (§6):** All domain invariants and the documented load-time rule (checksum/mutation gate) are implemented: edge endpoints, document refs, acyclicity (no duplicate block in hierarchy), block-reference link + edge consistency (every `[text](block:uuid)` has matching edge and target in heap), name uniqueness (case-insensitive in validation), no headings outside fenced code.
- **Document acyclicity:** “No block is its own ancestor” implemented as “no block appears more than once in root/sections/subsections”; matches the spec’s structure.
- **delete_block_safe:** Uses `incoming(block_id)` to reject; only removes edges where `source == block_id` in practice (because incoming is empty when we proceed). Behavior correct.
- **Checksum structure:** Blocks, edges, documents order and format match spec; timestamps excluded; `names.json` excluded.

---

## 5. Compliance suite gaps (missing cases)

The suite has **26 mutation scenarios** (up from 20 after Phase 1+2) and several **invalid fixtures** (load-time validation). Gaps below are closed unless marked optional.

### Mutation scenarios (command → expected result + assertions)


| Gap                         | Scenario idea                                                           | Status                                                                               |
| --------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| ~~Case-insensitive name~~   | `add-block-duplicate-name-case-insensitive.json`                        | **Done** (Phase 1)                                                                   |
| ~~Case-insensitive rename~~ | `rename-block-duplicate-name-case-insensitive.json`                     | **Done** (Phase 1)                                                                   |
| ~~Reserved characters~~     | `add-block-reserved-bracket.json`, `rename-block-reserved-bracket.json` | **Done** (Phase 2)                                                                   |
| ~~Encoding round-trip~~     | `add-block-special-chars.json`                                          | **Done** (Phase 2) — percent-encoding for filenames; `%` in names rejected outright. |
| **Long name truncation**    | AddBlock with name >200 bytes (after encoding) → success                | Open                                                                                 |
| ~~Commit model / expected_checksum~~ | CLI passes `base_checksum` to gate; harness passes current manifest checksum (or scenario `client_base_checksum`). `add-block-stale-state.json` expects rejected with StaleState when base ≠ vault. | **Done** — CLI fixed; spec scenario + harness gate |
| ~~AppendSection duplicate~~ | `append-section.json`, `append-section-duplicate-root.json`             | **Done** (Phase 2)                                                                   |
| ~~Manifest after mutation~~ | CLI integration test only (documented decision)                         | **Done** (Phase 2)                                                                   |


### Checksum / normalization (unit or compliance)


| Gap     | What to test                                                     | Status             |
| ------- | ---------------------------------------------------------------- | ------------------ |
| ~~NFC~~ | Same logical content in NFC form produces same checksum.         | **Done** (Phase 3) |
| ~~LF~~  | Content with `\r\n` normalized to `\n` produces stable checksum. | **Done** (Phase 3) |


### Invalid fixtures (load-time validation)

Existing: `duplicate-name`, `bad-checksum`, `dangling-uuid`, `duplicate-uuid`, `heading-in-block`, `missing-frontmatter`. Added:

- **Reserved in name** — DONE: `reserved-in-name` fixture; block name contains `[` or `]` → expected violation; `invalid_reserved_in_name_detected` in invariants test.

### Recovery / journal

- **Case A** — DONE: CLI integration test `recovery_case_a_rewrites_manifest_when_writes_landed`.
- **Case C** — DONE: CLI integration test `recovery_case_c_undoes_partial_writes`.
- **Journal format** — DONE: Round-trip tests for journal (including Name entries) in core.

### Summary

Phases 1–4 and the main Phase 5 items are done. Remaining **optional**: long name truncation scenario (AddBlock name >200 bytes after encoding → success). Harness and assertion types in good shape.

---

## 6. Summary


| Category                   | Status                                                                                                                                                      |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Critical correctness (1.1) | **Done** — case-insensitive name at command time                                                                                                            |
| Recovery Case C            | **Done** — undo on partial writes; error if journal corrupt (skipped undo steps). Design note: reattempt-then-undo remains implementation-defined optional. |
| Spec/interop (2.1–2.3)     | **Done** — NFC + LF normalization (+ tests); spec Rule 9 (fix on next write); journal Name format + round-trip                                              |
| Softer gaps (3.1, 3.2)     | **Done** — AppendSection ancestor check; cascade updates documents (remove section/subsection or delete doc)                                                |
| Recovery tests (Case A/C)  | **Done** — CLI integration tests                                                                                                                            |
| Invalid fixtures           | **Done** — reserved-in-name added                                                                                                                           |


**Remaining (optional):** Long name truncation scenario; suite coverage doc/README if desired.

---

## 7. Plan (prioritized)

### Phase 1 — Critical correctness (block non-conforming state) — DONE

1. **Case-insensitive name at command time (1.1)** — Implemented.
  - `NameIndex::resolve_ignore_case(name) -> Option<(String, Uuid)>` added; implemented in `InMemoryNameIndex` and `FsNameIndex`.  
  - `add_block` and `rename_block` use it; unit tests and compliance scenarios added.

### Phase 2 — Compliance coverage (lock in behavior) — DONE

1. **Reserved characters in names** — Implemented.
  - `add-block-reserved-bracket.json`, `rename-block-reserved-bracket.json` (expect rejected). Domain already rejected; no code change.
2. **Encoding and long names** — Implemented.
  - `add-block-special-chars.json`: AddBlock name `"Notes: Part 1"` → success + `block_name_is` assertion.
3. **AppendSection ancestor (3.1)** — Implemented.
  - `DomainError::BlockAlreadyInDocument(Uuid)`; `append_section` checks before calling domain.  
  - `append-section.json` (block already subsection → rejected), `append-section-duplicate-root.json` (block is root → rejected).
4. **Manifest after mutation** — Documented
  - **Decision:** Keep as CLI integration test only (`manifest_checksum_updated_after_add_block`). Adding a compliance assertion type would require the harness to compute checksums and pull domain logic into the test runner. The CLI test covers the commit protocol end-to-end.  
  - No new compliance scenario added.

### Phase 3 — Spec alignment and interop — DONE

1. **Checksum normalization (2.1)** — Unit tests and implementation: NFC/LF in `checksum::compute` and format serialize/parse; add_block/rename_block NFC-normalize names.
2. **Journal Name format (2.3)** — `BeforeImageEntry` serializes Name with top-level `kind`, `name`, `id`; round-trip tests added.
3. **Spec change: Rule 9 (2.2)** — Spec updated: correct on next write; checksum guards tampering.

### Phase 4 — Recovery and cascade — DONE

1. **Recovery Case A / C** — CLI integration tests: `recovery_case_a_rewrites_manifest_when_writes_landed`, `recovery_case_c_undoes_partial_writes`.
2. **Cascade and documents (3.2)** — Cascade updates documents (remove section/subsection or delete doc); spec and ARCHITECTURE updated; use case and domain helpers implemented.

### Phase 5 — Cleanup and docs — DONE (optional items open)

1. **Invalid fixtures**
  — `reserved-in-name` fixture and `invalid_reserved_in_name_detected` test added.  
    **Optional:** Long name truncation scenario; suite coverage README.

**Status:** Phases 1–5 complete for correctness and interop. Optional: long name truncation scenario, suite coverage doc.