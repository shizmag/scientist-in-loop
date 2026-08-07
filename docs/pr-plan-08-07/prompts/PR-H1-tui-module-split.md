# PR-H1 — TUI module split

## Role

Ship ONLY PR-H1. After F2+F3 preferred.

## Goal

Behavior-preserving split of `app.rs` (~4671 LOC) and `ui.rs` (~2100 LOC) into modules (jobs, help, tabs/*, bib_actions, similarity).

## Requirements

1. No intentional behavior change.
2. All existing tests compile and pass.
3. Prefer LOC per file < ~1500.
4. Clippy clean.

## Out of scope

- Keybinding redesign; new features

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Module map, residual risk.
