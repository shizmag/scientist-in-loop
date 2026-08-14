# PR-D4 — Mouse dispatch (slip-ok)

Copy the block below into an agent session. **After D1.** This PR is **slip-ok**.

---

## Role

You are the **mouse engineer** for scientist-in-loop. Ship ONLY PR-D4.

## Goal

Enable crossterm mouse capture. Clicks dispatch the same `CommandId`s / selection as keys. Tabs, list rows, footer chips. No drag-resize.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.15, KD-5
- Today: keys only. Event loop is in `crates/sil-tui/src/main.rs` / `lib.rs`.
- Enable on enter, disable on exit (restore terminal).

## Shared invariants

1. Minimal diff. Ignore mouse-move floods (only Down / Drag if needed for selection, prefer Down+Up).
2. Never auto-commit.
3. Keyboard still works if mouse is unsupported.
4. Clippy clean.

## Requirements

1. `EnableMouseCapture` / disable in the terminal setup/teardown.
2. Click zones:
   - Tab bar → change `active_tab`
   - Current list → change selection index
   - Double-click (or second click) → Enter / Open
   - Footer hint chips if they have hit boxes; otherwise skip
3. Unit-level: feed a `MouseEvent` into a handler; asserting `active_tab` changes is enough (no integration TTY).
4. Do not rebind keys.

## Out of scope

- Drag to resize panes
- Text selection / clipboard
- Touch gestures

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Which widgets are clickable, how events are tested, teardown restores terminal.
