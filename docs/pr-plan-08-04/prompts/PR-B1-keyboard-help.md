# PR-B1 — Keyboard help & documentation truth

Copy the block below into an agent session. Can run in parallel with A1/C1.

---

## Role

Focused implementer. Ship ONLY PR-B1. Can run in parallel with A1/C1 (touches mostly TUI UI + README).

## Goal

Make keybindings discoverable and fix stale help surfaces so keys shown match code.

## Repo context

- Tabs in code: 1 Dashboard | 2 Sources | 3 References | 4 Paper Draft | 5 Settings
- `crates/sil-tui/src/app.rs` key handlers; `ui.rs` draw/footer
- Footer titled roughly “Status & Help” but shows only status
- Dashboard panel “Daily Scientist Helper Shortcuts” is stale (4 tabs)
- README TUI section outdated (tab numbers, a/A keys)
- References right pane title may advertise `[t]itle` sort but Normal mode may not bind `t`
- No `?` overlay exists today

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Do not invent full keybinding remaps — document collisions only.
3. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. Implement mode-aware help overlay on `?` (and optionally F1):
   - Modes: Dashboard, Sources list, Reading MD, Viewing source refs, References left, References right, Settings, modals as needed
   - Content: actual keys from code (not aspirational)
2. Footer: keep status; add compact context key hints OR rename panel so it is not fake “Help”
3. Fix Dashboard shortcuts panel to 5 tabs + real Sources/References keys (`b`, `p`, `P`, `v`, etc.)
4. Fix README TUI documentation to match code
5. Either bind `t` for title sort in References Normal mode OR remove `[t]itle` from the pane title
6. Prefer small pure function `keymap_for(mode) -> Vec<(key, action)>` for testability; unit test a couple modes

## Out of scope

- Full keybinding redesign / resolving all `s`/`v` collisions (document only)
- Background job chrome (PR-B2)
- Binding orphan bib-delete unless one-line safe fix (prefer leave for later)

## Verify

```bash
cargo test -p sil-tui --lib
cargo clippy -p sil-tui --all-targets -- -D warnings
```

Manual smoke: open `sil tui`, press `?` on Sources and References.

## Deliverable

Keymap list per mode, README/Dashboard fixes, screenshots optional.
