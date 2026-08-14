# PR-D2 — Empty states / next-command chips

Copy the block below into an agent session. **After D1 + T2.**

---

## Role

You are the **empty-state engineer** for scientist-in-loop. Ship ONLY PR-D2.

## Goal

When a list is empty or stalled, show a short factual next action that names a `CommandId` (chip / line), not a blank pane and not a coaching essay.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.2, KD-2
- Dashboard already has empty copy for digest/ideas (Stage 13). Sources/refs/draft often do not.
- Command IDs live in D1. Dispatch chips by calling `App::dispatch`.
- ADR-015 KD-4: no “you should write the intro” coaching. Counts and next commands only.

## Shared invariants

1. Minimal diff. Same 2×2 dashboard layout. Same five tabs.
2. Never auto-commit.
3. Factual copy only.
4. Clippy clean on `sil-tui`.

## Requirements

1. Empty / stalled copy + chip → CommandId:

   | Surface | Copy | Command |
   |---------|------|---------|
   | Dashboard digest empty | set query in Settings or Refresh digest | `RefreshDigest` if implemented, else point at Settings tab |
   | Sources none | Drop a PDF in `sources/` or Fetch by DOI | `AddSourceLink` |
   | Sources all/some unparsed | `N unparsed — Parse selected / Parse all` | `ParseSelected` / `ParseAll` |
   | Refs right empty | Extract refs from a parsed source (Sources → v) | no chip required |
   | Draft no `\section` | Open in `$EDITOR` | existing external-editor path / `OpenExternalEditor` if registered |

2. Chips are keyboard-reachable: highlight + Enter, or a listed key. Do not require mouse (D4).
3. Do not add SQLite columns or new tabs.
4. Unit tests:
   1. Empty sources fixture renders fetch copy (string present in a testable model or draw helper).
   2. N unparsed > 0 shows parse copy.
   3. Non-empty sources list does **not** show the “no sources” empty state.

## Out of scope

- Wizard (O1), mouse (D4), fetch+parse policy (R1), badges (R3)

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Which panes got empty states, which CommandIds they dispatch, copy strings.
