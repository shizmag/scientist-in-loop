# PR-O2 — `sil init --demo` (slip-ok)

Copy the block below into an agent session. **After O1.** This PR is **slip-ok**.

---

## Role

You are the **onboarding engineer** for scientist-in-loop. Ship ONLY PR-O2.

## Goal

`sil init --demo [name]` creates a normal sil project plus a **synthetic** fixture so the dashboard, sources, ideas, and bib are non-empty. No copyrighted PDFs. No network.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.13, KD-19
- Today: `sil init` scaffolds empty stubs (`crates/sil/src/commands/init.rs`, `crates/sil/src/templates.rs`).
- E2E: `crates/sil/tests/e2e_init.rs`.

## Shared invariants

1. Minimal diff. Call existing init, then overlay demo files.
2. Never auto-commit (still a commit **proposal** only).
3. Synthetic markdown source only (not a real paper).
4. Clippy clean.

## Requirements

1. Flag `--demo` on `sil init`.
2. After init:
   - Write `sources/demo-attention.md` (short fake notes about “Demo Attention”)
   - Parse it into SQLite (reuse parse; md native — no Marker/PDF)
   - Write a 2-section `paper_draft.tex` with one `# -- X -- #` idea and a `\cite{demo2024}`
   - Write a matching `references.bib` entry
3. Wizard (O1) may offer “Create demo project” if cheap; not required.
4. Tests:
   1. e2e `sil init --demo` in temp dir: source parsed, draft contains `# -- X -- #`, bib has the key, no network.
   2. Existing `sil init` without `--demo` unchanged.

## Out of scope

- Releases / install.sh
- Real PDF fixtures
- Auto-opening the TUI

## Verify

```bash
cargo test -p sil --test e2e_init
cargo clippy -p sil --all-targets -- -D warnings
```

## Deliverable

Flag, fixture paths, e2e name, residual “Marker unused”.
