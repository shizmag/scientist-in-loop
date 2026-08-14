# PR-W4 — Proposal / uncommitted diff (slip-ok)

Copy the block below into an agent session. **After W2 + T1.** This PR is **slip-ok**.

---

## Role

You are the **diff engineer** for scientist-in-loop. Ship ONLY PR-W4.

## Goal

Thin review desk: show `git status` + uncommitted diff of `paper_draft.tex` / `references.bib` + last Sci-Action proposal text. Actions: copy/show proposal. Discard TUI-originated edits via **undo (T1)** only. **Never `git commit` / `git checkout`.**

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.14, KD-23
- Today: `sil git propose` prints a proposal. TUI has no diff. `sil-git` crate has status/propose helpers.

## Shared invariants

1. Minimal diff. Read-only git.
2. **Forbidden:** `git commit`, `git checkout`, `git restore`, `git reset`.
3. Register `ReviewChanges` on the palette.
4. Clippy clean.

## Requirements

1. Modal with:
   - porcelain status (or `sil-git` status)
   - unified diff for draft + bib if dirty
   - latest proposal text from existing propose helper (in-memory; do not commit)
2. Actions:
   - Copy proposal to clipboard if a crate already used by the workspace can do it; else write `.sil/last_proposal.txt` via atomic write and status the path
   - Discard: call T1 undo if the last generation exists; otherwise status “use git yourself — sil will not checkout”
3. T4 may link “View diff” to this modal if T4 stubbed it.
4. Unit tests:
   1. Modal build from fixture strings does not invoke git commit.
   2. Grep the PR diff: no `git commit` / `checkout` / `restore` in `sil-tui` / `sil-app`.
   3. Discard without undo journal is a no-op with status.

## Out of scope

- Hunk-level staging
- Graphite / GitHub PR
- Auto-commit

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
rg -n 'git commit|git checkout|git restore|git reset' crates/sil-tui crates/sil-app && exit 1 || true
```

## Deliverable

Diff source, proposal copy path, discard semantics.
