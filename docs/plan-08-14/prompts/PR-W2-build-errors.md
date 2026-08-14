# PR-W2 — Build job + first-error jump

Copy the block below into an agent session. **After D1.**

---

## Role

You are the **build engineer** for scientist-in-loop. Ship ONLY PR-W2.

## Goal

Run the existing LaTeX build as a non-blocking TUI job. On failure, parse the first `file:line` from the engine log / stderr and jump the Draft tab viewer to that line.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.14, KD-21
- Today: `sil paper build` is CLI (`crates/sil/src/commands/build.rs`, `sil-latex` engine). TUI has no build job.
- Reuse compile APIs from `sil-latex`. Panic-isolate the worker like other jobs.

## Shared invariants

1. Minimal diff. No new engines.
2. Never auto-commit.
3. Register `BuildDraft` on the palette.
4. Clippy clean.

## Requirements

1. `JobKind::Build` (or reuse a generic job) wrapping existing compile of `config.latex.main`.
2. Success: status with output PDF path.
3. Failure: parse first error location from log/stderr (`file:line` / `file:line:col`). Jump `active_tab` to Paper Draft and set scroll/section so that line is visible (clamp if out of range).
4. Pure helper `fn parse_latex_error_location(log: &str) -> Option<(String, usize)>` — unit test with fixture log snippets (tectonic / pdflatex style).
5. Missing engine → `UserError` `latex.engine_missing` if T2 landed.
6. Unit tests:
   1. Log parser extracts line from a fixture.
   2. Jump clamps when line > file length.
   3. Build does not run on the UI thread (spawned); in-flight flag prevents a second start.

## Out of scope

- Live PDF preview
- New TeX engines
- Release / journal zip (`sil paper build release`) unless calling the default draft build is enough — **draft build only**

## Verify

```bash
cargo test -p sil-tui -p sil-latex
cargo clippy -p sil-tui -p sil-latex --all-targets -- -D warnings
```

## Deliverable

Job wiring, log parser examples, jump behavior.
