# PR-A2 — Completeness-aware upsert + arXiv normalize

Copy the block below into an agent session. Assumes PR-A1 is already on the base branch.

---

## Role

Focused implementer. Ship ONLY PR-A2. Assumes PR-A1 (pretty_format) is already on the branch/main you base on.

## Goal

Fix `upsert_bib_entry` so it prefers complete entries over incomplete stubs, and harden paper identity matching for arXiv versions.

## Repo context

- `crates/sil-core/src/bib.rs`: `upsert_bib_entry`, `extract_bib_entry_info`, `is_same_paper`, `BibEntryInfo.is_incomplete`
- Incomplete heuristics today: notes/status containing unproved/incomplete, `journal={unknown}`, `author={unknown}`
- Docs currently claim “prefer complete” but code always replaces first match — align code to policy.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. Upsert decision matrix when `is_same_paper(existing, new)`:
   - incomplete existing + any new → replace
   - complete existing + incomplete new → KEEP existing (do not demote)
   - complete existing + complete new → replace (official upgrade OK)
   - no match → append
2. Normalize arXiv IDs in `is_same_paper` (and helpers if needed): strip `arxiv:` / `arXiv:` prefixes and trailing `v\d+` so `1234.5678v1` matches `1234.5678`.
3. Unit tests covering each matrix cell + arXiv version match.
4. Fix doc comments on `upsert_bib_entry` to match behavior.

## Out of scope

- Cite-key rewrite/preserve (PR-A3)
- TUI file locks (PR-A4)
- Network resolution changes (PR-C2)

## Verify

```bash
cargo test -p sil-core
cargo clippy -p sil-core --all-targets -- -D warnings
```

## Deliverable

Decision table implemented, tests listed, residual risks (e.g. two complete entries disagreeing on fields).
