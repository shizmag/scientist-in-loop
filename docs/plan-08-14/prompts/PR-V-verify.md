# PR-V — Verification stage

Copy the block below into an agent session. **After all code PRs (D*, R*, T*, O*, W*).** Do not start early.

---

## Role

You are the **verifier** for scientist-in-loop Stage 14. Ship ONLY PR-V. You do **not** add product features.

## Goal

Prove the wave is honest and green: workspace tests/clippy/fmt, scenario checklist against the plan, and a residual-risk note. Fix only **blocker bugs** the checklist proves (compile/test failures, panics, sixth tab, auto-commit, `git checkout` from TUI). Do not expand scope.

## Repo context

- Parent plan: `docs/plan-08-14/pr-plan.md` §9–§10
- Code PRs: D1–D4, R1–R4, T1–T6, O1–O3, W1–W4 (slip-ok D4/W3/W4/O2 may be absent — mark those checklist rows `skipped`)
- Check-work skill if present: use its spirit (build + tests + diff review), stay inside this prompt’s gates

## Shared invariants

1. No new commands, tabs, MCP tools, or settings keys except those the plan already shipped.
2. Never auto-commit.
3. If you must fix a blocker, keep the diff minimal and describe it in the deliverable.
4. Cosmetic README/STAGES edits belong in **Z**, not here — unless a test or honesty grep in this prompt fails.

## Requirements

### 1. Automated gate

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

All three must be green.

### 2. Honesty greps (code, not docs)

```bash
# Still five tabs
rg -n 'enum ActiveTab' -A 20 crates/sil-tui/src/app/types.rs
# Fail if a sixth variant appeared

# No git mutation from TUI/app
rg -n 'git commit|git checkout|git restore|git reset' crates/sil-tui crates/sil-app && exit 1 || true

# Palette exists
rg -n 'CommandId|OpenPalette|CommandPalette' crates/sil-tui/src

# Digest/add-source fetch parse=true (TUI path)
rg -n 'parse:\s*true|parse\s*=\s*true' crates/sil-tui/src

# repair-db must not wipe sources
rg -n 'remove_dir_all' crates/sil crates/sil-app crates/sil-db crates/sil-parse || true
# Inspect hits: none may target sources/

# MCP tool count still 6
rg -n 'sil_context|sil_sources|sil_cite|sil_draft|sil_review|sil_propose' crates/sil-mcp/src
```

### 3. Scenario checklist (throwaway project)

Create a temp project (`sil init` or `sil init --demo` if O2 shipped) **or** walk the code paths if a full TUI session is impractical. Mark each item pass / fail / skipped / blocked-how.

**Discoverability**

- [ ] `:` / `Ctrl-K` opens palette; Esc restores mode
- [ ] Empty sources shows a fetch/parse next action
- [ ] `1–5`, `?`, `q`, `j/k`, `Ctrl+S` still work
- [ ] Mouse clicks tabs (skip if D4 slipped)

**Reading loop**

- [ ] Digest Enter / add-source uses parse=true and does not auto-open reader
- [ ] Note `n` section picker; Esc does not write
- [ ] Sources badges: parsed / in bib / cited derived
- [ ] Cite-into-section inserts `\cite{key}` only in the chosen section

**Trust**

- [ ] Undo restores last bib/note mutation
- [ ] Failed job status is a human title, not `Debug` of anyhow
- [ ] Restarting TUI marks in-flight jobs stale (T3)
- [ ] Dirty + newer mtime blocks save (T4)
- [ ] Live other lock holder requires confirm (T6)
- [ ] `--repair-db` backups db; `sources/` file count unchanged

**Onboarding**

- [ ] `sil tui` without project opens wizard
- [ ] Doctor JSON has `hint` on a failed check
- [ ] `sil init --demo` is non-empty (skip if O2 slipped)

**Writing / agent**

- [ ] Open last estimate report (empty-state if none)
- [ ] Build failure jumps to a parsed line (or status if no log)
- [ ] Grounding modal does not write draft (skip if W3 slipped)
- [ ] Review-changes does not call git commit (skip if W4 slipped)

### 4. Residual-risk note

List slipped PRs, known holes (Windows rename, flock, split view), and any blocker you fixed.

## Out of scope

- Product features
- ADR/README (Z)
- Re-running slipped PRs

## Verify

The three cargo commands above + checklist filled.

## Deliverable

Gate transcript (pass/fail), checklist, residual-risk note, files you had to touch for blockers only.
