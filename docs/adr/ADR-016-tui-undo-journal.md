# ADR-016: TUI File Mutation Undo Journal

## Status
Accepted (Wave 08-14 / Stage 14, PR-T1 / KD-11)

## Context
In `sil-tui`, mutations such as deleting bibliography entries, parking notes into `paper_draft.tex`, or deleting sources modify durable files on disk. Previously, there was no undo mechanism for these destructive or insertion operations other than manual git reverts or file editing. Using `git checkout` / `git restore` would violate project invariants, wipe uncommitted user modifications across the repository, or conflict with dirty git worktrees.

## Decisions

1. **Dedicated Generation Journal (`.sil/undo/`)**: Implemented `UndoJournal` in `sil-core` that writes atomic snapshot records (`{id:06}.json`) under `<project_root>/.sil/undo/`.
2. **Non-Git Invariant**: The undo mechanism strictly copies exact file byte snapshots to and from disk without invoking `git` commands (`git checkout`, `git restore`, `git commit`).
3. **Bounded Journal Cap**: Enforced a strict maximum cap of **10** generations (`UndoJournal::MAX_GENERATIONS`). When the 11th generation is added, the oldest snapshot is automatically pruned from disk.
4. **Covered TUI Mutations**:
   - **Bib Delete**: Snapshots `references.bib` before removing the selected bib entry.
   - **Reader Note Insert**: Snapshots `paper_draft.tex` before parking note blocks into the manuscript.
   - **Source Delete**: Snapshots `references.bib` and any related files before source removal.
5. **Palette & Keybinding (`Ctrl+Z`)**:
   - Registered `CommandId::Undo` in the command palette.
   - Bound `Ctrl+Z` in `Normal` input mode to dispatch `CommandId::Undo`.
   - Avoided overriding `u` globally as `u` is used for "Use Cache" in the Settings tab.
6. **Project State Resynchronization**: Upon executing `Undo`, `App::dispatch` restores file bytes from the latest generation, pops the snapshot, refreshes in-memory data (`paper_draft.tex`, `references.bib`, draft section caches, SQLite TODO ideas), updates mtimes, and sets a descriptive status message (e.g. `Undone: Delete bib entry`).
7. **Managed `.gitignore`**: Added `.sil/undo/` to the `sil-managed` block in `.gitignore` templates so undo journal snapshots are never committed to version control.

## Residuals & Out of Scope

- **Redo UI Stack**: Redo UI polish is deferred beyond the core working undo stack.
- **Settings YAML Undo**: Undo focuses on durable project files (`references.bib`, `paper_draft.tex`); global and local settings undo is out of scope.
