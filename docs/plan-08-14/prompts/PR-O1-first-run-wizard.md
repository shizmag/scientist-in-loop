# PR-O1 — No-project first-run wizard

Copy the block below into an agent session. **After D1 + O3.**

---

## Role

You are the **onboarding engineer** for scientist-in-loop. Ship ONLY PR-O1.

## Goal

When `sil tui` starts with `project_root == None`, show a wizard: recent projects, open path, create project, run host doctor. A scientist should reach a real project without knowing clap flags.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.13, KD-18
- Today: `App::new(project_root: Option<...>)`. No-project still opens tabs with empty local settings.
- Recents: `GlobalSettings.recent_projects` (`sil-core` / `sil paper recent`). Cap 20 already exists. `touch_recent` on open should already be used by CLI — reuse it when the wizard opens a path.
- Init lives in `crates/sil/src/commands/init.rs` / `sil-core` templates. TUI must not fork layout rules. Prefer calling existing init helpers, not shelling a nested `sil` if a library path exists; if only the binary path exists, extract a small library call rather than `std::process::Command` when practical.

## Shared invariants

1. Minimal diff. Same five tabs **after** a project is chosen.
2. Never auto-commit (`sil init` still only **proposes** a commit).
3. Missing recent paths are skipped or shown as `UserError` (`project.not_found`), not a panic.
4. Clippy clean.

## Requirements

1. `InputMode::Wizard` (name may vary) when `project_root` is `None` at startup.
2. Menu:
   1. Recent projects (existing paths only; skip missing)
   2. Open path (modal, Utf8 path)
   3. Create project (name → existing `sil init` behavior)
   4. Run doctor (host checks only — git/python/latex — via existing doctor function)
3. Opening a project sets `project_root`, loads config/sources/bib (same as today’s project-mode boot), records recent.
4. Esc on a sub-modal returns to the wizard list. Quit (`q`) still exits the TUI.
5. Unit tests:
   1. `App::new(None)` starts in Wizard, not Dashboard-as-if-project.
   2. Missing recent path does not panic; status is a `UserError` title or the path is omitted.
   3. Selecting a temp dir that **is** a sil project (config.yaml present) leaves Wizard and has `project_root.is_some()`.

## Out of scope

- `sil init --demo` (O2)
- Rewriting install.sh / Releases
- Auto-opening last recent without a choice

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Wizard modes, how init is invoked, how recents are filtered, residual “create project from TUI still proposes commit”.
