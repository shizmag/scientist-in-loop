# PR-W1 — Estimate report viewer

Copy the block below into an agent session. **After D1.**

---

## Role

You are the **estimate engineer** for scientist-in-loop. Ship ONLY PR-W1.

## Goal

Expose the existing L0 estimate job in the palette and add “Open last review” so a scientist can read `.sil/reviews/*/report.md` (or JSON summary) inside the TUI. Do not change scoring. Do not claim peer-review truth.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.14, KD-20
- Today: `App::run_estimate_job` already runs `sil_agent::run_heuristic_estimate` (quick mode) on a background thread. Result is a status line + job history. No report modal.
- Reviews live under `.sil/reviews/` when `--write` / estimate write path is used. If the current TUI job does **not** write a report, add an optional write (same as CLI `--write`) **or** render the in-memory report in the modal. Prefer: write under `.sil/reviews/` via existing agent API if it exists and is read-only on the draft.

## Shared invariants

1. Minimal diff. Do not change L0 heuristics.
2. Never auto-commit. Estimate must not write `paper_draft.tex`.
3. Register `RunEstimate` + `OpenLastReview` on the palette.
4. Clippy clean.

## Requirements

1. Palette / documented key to run the existing estimate job (do not block the UI).
2. `OpenLastReview`: find newest `.sil/reviews/*/report.md` or `report.json`. Modal = scrollable text. Empty: “no reviews yet — run Estimate”.
3. If the job currently does not persist a report, persist one using the existing write helper so Open Last works after a TUI run.
4. Unit tests:
   1. Missing reviews dir → empty-state copy, no panic.
   2. A fixture `report.md` is opened and appears in the modal buffer.
   3. Running estimate does not set `reading_md_content` / does not dirty the draft.

## Out of scope

- Multi-persona L1 LLM calls
- Changing rubrics
- Auto-opening the report on job complete (status “open from palette” is enough; auto-open is OK if cheap and does not steal focus from typing)

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

How reports are written/found, modal mode, palette IDs.
