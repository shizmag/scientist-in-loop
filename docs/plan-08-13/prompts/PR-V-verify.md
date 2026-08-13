# PR-V — Verification stage

Copy the block below into an agent session. **After A1, B1, B2, C1, C2, C3.** Do not start early.

---

## Role

You are the **verifier** for scientist-in-loop Stage 13. Ship ONLY PR-V. You do **not** add product features.

## Goal

Prove the wave is honest and green: workspace tests/clippy/fmt, scenario checklist against the plan, and a residual-risk note. Fix only **blocker bugs** that the checklist proves (compile/test failures, panics, dummy dashboard strings still present). Do not expand scope.

## Repo context

- Parent plan: `docs/plan-08-13/pr-plan.md` §9–§10
- Code PRs: A1 live dashboard, B1 settings, B2 background digest, C1 reader `b`, C2 reader `n`, C3 digest Enter
- Check-work skill if present: `~/.grok/skills/check-work/SKILL.md` — use its spirit (build + tests + diff review), but stay inside this prompt’s gates

## Shared invariants

1. No new commands, tabs, MCP tools, or settings keys.
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

### 2. Honesty grep (code, not docs)

```bash
# Dashboard must not still be a mock
rg -n 'Quantum Advantage|self-attention baseline|Stage 5 \(Polish' crates/sil-tui/src/ui/dashboard.rs && exit 1 || true

# No sil daily command leaked into clap
rg -n 'enum Commands' -A 80 crates/sil/src/cli.rs
# Fail if a Daily variant exists

# Reader help mentions b and n
rg -n 'ReadingSourceMd' -A 20 crates/sil-tui/src/app/types.rs
```

### 3. Scenario checklist (throwaway project)

Create a temp project (`sil init` in a tmp dir) **or** walk the code paths if a full TUI session is impractical. Mark each item pass / fail / blocked-how.

**Dashboard (A1)**

- [ ] Health stage matches `config.yaml` `project.stage`, not “Stage 5”
- [ ] Bib coverage is a real `cited/total` (or empty-bib message)
- [ ] Label line is not hardcoded OK when the draft has a broken `\ref`
- [ ] Ideas pane shows a real `# -- X -- #` after you insert one in `paper_draft.tex` and reload (`R`)
- [ ] Digest pane shows DB rows or the empty-state — not dummy Nature/IEEE titles
- [ ] Pane 4 still shows the keymap

**Settings + digest (B1/B2)**

- [ ] Settings tab exposes digest query (global + local) and refresh hours
- [ ] Save (`Ctrl+S`) writes YAML; reopen sees the values
- [ ] Old project without those keys still opens
- [ ] Empty query does not spawn a digest job
- [ ] With a query and empty/stale cache, opening Dashboard queues at most one digest job (`J`)

**Reader (C1/C2)**

- [ ] `Enter` on a parsed source still opens the reader; `q` exits
- [ ] `b` in the reader upserts `references.bib` (draft marker) without leaving the reader
- [ ] `n` + a one-line note inserts `# -- X -- #` with `from: <filename>`
- [ ] Esc / empty note does not write the draft
- [ ] No `git commit` appeared

**Digest inbox (C3)**

- [ ] j/k moves the digest highlight on Dashboard
- [ ] Enter on a DOI row queues fetch (same chrome as Sources `a`)
- [ ] Enter on a row without DOI/URL is a status error
- [ ] Fetch does not auto-open the reader or auto-parse

### 4. Residual risk note (required)

List anything still true from plan §11, plus any new residuals you found (e.g. “digest job untested against live Crossref”).

## Out of scope

- ADR / STAGES / README (Z)
- New features to “complete” residuals
- Golden-dataset / ONNX / MCP tool-count work unless tests you ran failed because of this wave

## Verify

The commands in §1 plus a written checklist with pass/fail.

## Deliverable

1. Command results (ok / fail)
2. Checklist table
3. Blocker fixes (if any) + files
4. Residual risk
