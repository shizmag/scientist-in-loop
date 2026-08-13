# PR-B1 — Digest settings

Copy the block below into an agent session (worktree-isolated if parallel with A1).

---

## Role

You are the **settings engineer** for scientist-in-loop. Ship ONLY PR-B1.

## Goal

Persist a digest query and refresh interval in settings the user can edit on TUI tab 5. Old YAML without these keys must still load.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-13/pr-plan.md` §5.2, KD-5, KD-6
- Types: `crates/sil-core/src/settings.rs` — `GlobalSettings`, `LocalSettings`
- Local settings live on `Config.settings` (`crates/sil-core/src/config.rs`)
- TUI fields: `GlobalField` / `LocalField` / `setting_items()` in `crates/sil-tui/src/app/types.rs` and `bib_actions.rs`
- Settings UI: `crates/sil-tui/src/ui/settings.rs`
- Save path: `App::save_all` already writes global settings + local config atomically

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. `serde` defaults so existing `settings.yaml` / `.sil/config.yaml` keep loading.
4. No digest HTTP and no TUI job in this PR (B2).
5. Prefer unit tests co-located; clippy clean on touched crates.

## Requirements

1. Add to `GlobalSettings`:
   - `digest_query: String` (default `""`)
   - `digest_refresh_hours: u32` (default `1`)
2. Add to `LocalSettings`:
   - `digest_query: String` (default `""`) — when non-empty, this **wins**
3. Add `sil_core` helper, e.g. `effective_digest_query(global, local) -> Option<&str>`:
   - local trimmed non-empty → `Some(local)`
   - else global trimmed non-empty → `Some(global)`
   - else `None` (auto-refresh disabled)
4. Clamp refresh hours: treat `0` as `1` on load and on save (KD-6). Helper `effective_digest_refresh_hours(hours) -> u32`.
5. TUI Settings tab:
   - Show a **Digest** divider (same visual language as Global / RAG / Local).
   - Editable fields: global query, refresh hours, local query override.
   - Extend `GlobalField` / `LocalField` (or a tiny `DigestField`) and `setting_items()`.
   - Enter / `e` edits like other string/number fields.
   - `Ctrl+S` / `s` persists via existing `save_all`.
6. Unit tests:
   1. Default `GlobalSettings` / `LocalSettings` deserialize from `{}`.
   2. YAML missing digest keys → defaults.
   3. Effective query precedence (local wins; both empty → `None`).
   4. `digest_refresh_hours: 0` clamps to 1.

## Out of scope

- Background fetch job (B2)
- Dashboard rendering (A1)
- Changing `sil source digest` CLI flags
- Multi-query watch lists
- Daemon / cron

## Verify

```bash
cargo test -p sil-core -p sil-tui
cargo clippy -p sil-core -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Field names, effective-query helper signature, Settings tab labels, residual “refresh not wired until B2”.
