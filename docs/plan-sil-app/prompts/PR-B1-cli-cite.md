# PR-B1 — CLI cite append / promote via sil-app

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **CLI engineer** for scientist-in-loop. Ship ONLY PR-B1.

## Goal

Make `sil source cite --append` and `sil source cite --promote` call `sil-app`. Suggestion-only cite (no flags) stays as-is. CLI success output stays quiet: ✓ / warn only — **no** git proposal block (KD-8).

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.5, §6 B1, KD-8
- Prerequisite: **PR-A1 merged** (`sil-app` exists)
- Today: `crates/sil/src/commands/cite.rs`
  - `--promote` inlines parse/unmark/write
  - `--append` calls `sil_core::bib::upsert_bib_entry` (no `preserve_cite_key`)
- E2E: `crates/sil/tests/e2e_cite.rs` (does not assert proposal stdout)

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Do **not** print `CommitProposal::display()` / “Proposed commit” for cite.
4. `draft=false` on CLI append (official / user-requested append).
5. Do not change suggestion-only behavior (`--json`, filename, query, official resolve for display).

## Requirements

1. Add `sil-app` to `crates/sil/Cargo.toml` dependencies.
2. `--promote` branch:
   - `AppContext::from_root` (or `from_cwd` via existing `load_project`)
   - `promote_bib(ctx, PromoteBib { target })`
   - On success: same style ✓ message as today (`Promoted entry '{key}' in {path} ...`)
   - On error: `bail!` / anyhow with the use-case error
3. `--append` branch (after suggestion is computed, existing logic):
   - `upsert_bib(ctx, UpsertBib { entry: suggestion.bibtex, draft: false })`
   - ✓ “Updated existing entry” vs “Appended entry” from `replaced`
4. Behavior change (intended, KD-5): append now **preserves** cite key when replacing the same paper.
5. Do not add `--draft` or `--preserve-cite-key` flags.
6. `e2e_cite` remains green. Do not require new proposal text in stdout.
7. Optional (nice): a small e2e that `--append` of a same-DOI entry keeps the old key — only if cheap; not required.

## Out of scope

- MCP / TUI
- `source fetch`
- Printing Sci-Action proposals on cite
- Changing `suggest_from_*` / official metadata resolution used for **display**
- STAGES / ADR

## Verify

```bash
cargo test -p sil --test e2e_cite
cargo clippy -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Files changed, before/after cite write path, confirmation that stdout has no “Proposed commit”.
