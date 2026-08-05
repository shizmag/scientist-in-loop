# PR-A4 — Hydration races & write safety (TUI)

Copy the block below into an agent session. Base on A1–A3.

---

## Role

Focused implementer. Ship ONLY PR-A4. Base on A1–A3.

## Goal

Eliminate promote/hydrate races and concurrent references.bib last-writer-wins bugs in the TUI; improve hydration dedup and write-failure UX.

## Repo context

- ADR-009: `docs/adr/ADR-009-background-bibtex-hydration.md`
- Implementation: `crates/sil-tui/src/app.rs` — `queue_ref_hydration`, `queue_source_hydration`, `poll_background_hydration`, `in_flight_hydration_keys`
- Promote: `promote_selected_bib_entry` unmarks `% [sil: tui-added]`
- Bug: hydration success always re-applies `mark_tui_added` even if user already promoted.
- Bug: multiple threads read-upsert-write without serialization.
- Source dedup: DOI or source_id only — missing pure arXiv key for sources.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI bib add stays non-blocking (local first + background hydrate).
3. Release strip only removes `% [sil: tui-added]` blocks from packages.
4. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. Serialize all TUI writes to `references.bib` (single writer queue or mutex around read-modify-write). Background workers must only return resolved bib strings; main/poll thread performs disk write.
2. On hydration success, re-read current file, find matching entry:
   - If still tui-added → upgrade fields (preserve cite key per A3) and keep marker.
   - If already unmarked (promoted) → upgrade fields if completeness allows but DO NOT re-mark.
   - If entry deleted → no-op / status note.
3. Dedup keys for source hydration: include `arxiv:{id}` when present.
4. If disk write fails after successful fetch, set a clear error status (do not silent-fail).
5. Unit tests: promote-during-flight simulation; dedup key for arXiv-only source; write path behavior.

## Out of scope

- Job status chrome UI (PR-B2) — keep existing status strings if needed
- CLI multi-process locking across processes (in-process TUI only)

## Verify

```bash
cargo test -p sil-tui --lib
cargo test -p sil-core
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Race scenarios covered, files touched, residual multi-process risk note.
