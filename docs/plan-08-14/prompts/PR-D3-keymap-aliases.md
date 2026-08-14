# PR-D3 — Keymap aliases via registry

Copy the block below into an agent session. **After D1.** Slip-ok: no, this is cheap polish — keep unless time-boxed after D4/W3/W4.

---

## Role

You are the **palette engineer** for scientist-in-loop. Ship ONLY PR-D3.

## Goal

Generate the `?` help overlay from the `CommandId` registry. Document contextual collisions (`v` is refs / `$EDITOR` / venue). Do **not** mass-rebind keys. Palette remains the escape hatch.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.15, KD-4
- Today: `keymap_for(HelpMode)` is hand-maintained strings. D1 added `CommandSpec.default_keys`.

## Shared invariants

1. Minimal diff. No muscle-memory break (`1–5`, `?`, `q`, `j/k`, `Ctrl+S` stay).
2. Never auto-commit.
3. Clippy clean.

## Requirements

1. `keymap_for` (or replacement) lists CommandId titles + keys for the current `HelpMode`.
2. Collision note for contextual `v` / `p` / `P` where they differ by tab.
3. Palette search aliases include those titles.
4. Unit test: help text for `HelpMode::ReadingSourceMd` includes cite/note (and cite-into-section if R4 merged).
5. Unit test: `ActiveTab` count still 5.

## Out of scope

- Mouse (D4)
- Rebinding `v` to a single meaning

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

How help is generated, collision notes added.
