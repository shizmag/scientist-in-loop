# PR-D1 — Command registry + palette

Copy the block below into an agent session (worktree-isolated if parallel with T2).

---

## Role

You are the **palette engineer** for scientist-in-loop. Ship ONLY PR-D1.

## Goal

Introduce a `CommandId` registry and a command palette (`:` / `Ctrl-K`) so scientists can find actions by name. Existing keys keep working. New verbs in later PRs will `dispatch` the same IDs.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.1, KD-1, KD-2, KD-3, KD-4
- Today: `crates/sil-tui/src/app/handlers/mod.rs` is a large `handle_key` match. Help is `keymap_for(HelpMode)` in `crates/sil-tui/src/ui/mod.rs` / `app/types.rs`.
- Footer cheatsheet already exists. `?` / `F1` overlay exists.
- No `CommandId`, no palette modal, no `App::dispatch`.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Never auto-commit.
3. Same five tabs. No sixth tab. No mouse in this PR (D4).
4. Do **not** mass-rebind keys. Map a first set of globals + high-value verbs onto `dispatch`; leftover keys may stay inline until D3.
5. Prefer unit tests co-located; clippy clean on touched crates.

## Requirements

1. Add `CommandId` (enum) + `CommandSpec { id, title, aliases, default_keys, tab }` in `sil-tui` (do **not** put Ratatui types in `sil-core`). If T2 wants a retry string, expose `CommandId::as_str()`.
2. `App::dispatch(CommandId)` is the single run entry. At minimum wire:
   - `OpenPalette`, `SaveAll`, `Quit`, `OpenHelp`, `Reload`, `OpenJobHistory`
   - `ParseSelected`, `ParseAll`, `AddSourceLink`, `OpenSource`
   - `CiteSource` (existing reader/list `b` path), `CaptureNote` (existing `n` path)
   - Stubs allowed for commands later PRs will fill (`FetchParse`, `CiteIntoSection`, `Undo`, `RunEstimate`, `OpenLastReview`, `BuildDraft`, `RepairDb`, `RefreshDigest`) — stub = status “not implemented yet” **or** omit from the visible catalog until implemented. Prefer **register only what runs**.
3. Palette UI (`InputMode::CommandPalette` or equivalent):
   - Open with `:` or `Ctrl-K` from Normal (and other non-text-entry modes).
   - Filter on title + id + aliases (case-insensitive substring is enough; fuzzy nice-to-have).
   - `j`/`k` or arrows move; Enter runs; Esc restores previous `InputMode`.
   - Unavailable commands hidden or dimmed with a one-line reason (“not in a project”).
4. `handle_key` for the first set above calls `dispatch` instead of duplicating bodies.
5. Help overlay / footer may mention `:` / `Ctrl-K`. Do not rewrite every contextual keymap (D3).
6. Unit tests (no extra crates):
   1. Filter `"parse"` lists parse commands and not unrelated ones.
   2. Esc from palette restores previous mode.
   3. `dispatch(SaveAll)` is the same effect as `Ctrl+S` (dirty cleared / save path invoked — mock or spy as the existing tests do).
   4. Opening palette does not quit the app (`q` is not intercepted as quit while palette is open).

## Out of scope

- Mouse (D4), empty-state chips (D2), keymap collision rewrite (D3)
- Fetch+parse policy change (R1)
- UserError catalog (T2)
- New CLI commands, MCP tools

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

Confirm `:` / `Ctrl-K` open the palette in a unit-level key test.

## Deliverable

Files changed, `CommandId` list actually registered, which keys now go through `dispatch`, residual keys still inline.
