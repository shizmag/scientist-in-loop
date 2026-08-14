# ADR-016: Scientist-Facing TUI & Visible Robustness

## Status
Accepted (Wave 08-14 / Stage 14)

## Context
The five-tab TUI was a capable cockpit, but it was difficult to operate without prior knowledge: commands were hidden behind overloaded keys and there was no search palette. Reading also had dead ends: fetched sources were not parsed as part of the same workflow, notes defaulted to the end of the draft, and the reader could add a bibliography entry but not cite into a chosen section. Empty projects offered little guidance.

Failures and concurrent work were similarly difficult to see or reverse. Errors were mostly raw status strings, jobs lived only for the TUI process, the advisory workspace lock was silent, external edits had no conflict banner, and a corrupt SQLite index had no repair path. Writing tools existed in CLI or MCP surfaces but did not provide a thin review handshake inside the TUI.

## Decision
Stage 14 keeps the existing terminal product and five tabs, and makes its scientist-facing workflows findable, connected, and honest.

1. **KD-1–KD-5, command spine**: A `CommandId` registry carries titles, aliases, availability, and dispatch for palette, keyboard, mouse, and empty-state actions. The palette is available with `:` and `Ctrl-K`; existing navigation and keybindings remain. Mouse support is limited to practical selection and activation, not layout editing.
2. **KD-6–KD-7, fetch policy**: TUI source-link and digest ingestion use composite fetch+parse. Parsing does not auto-open the reader; opening remains an explicit command.
3. **KD-8–KD-9, reading loop**: Reader note capture offers draft sections or end-of-draft and records the selected section. Reader citation inserts a real existing cite key into a selected draft section, upserting the source first when needed. Neither operation auto-commits.
4. **KD-10, derived badges**: Sources render `parsed`, `in bib`, and `cited` facts derived from current files and in-memory records. No triage columns or workflow state are stored in SQLite.
5. **KD-11, undo journal**: TUI file mutations snapshot exact bytes in atomic generation records under gitignored `.sil/undo/`, retain the last 10 generations, and restore through `Ctrl+Z` or the palette. Undo is not Git history and does not invoke checkout, restore, or commit. It covers source/bibliography deletion and note/citation insertion where implemented.
6. **KD-12, user errors**: Failures are classified into `UserError { code, title, hint, retry }`; the TUI presents an actionable title while retaining detail for the error surface and machine-readable paths.
7. **KD-13, persistent jobs**: Fetch, parse, digest, estimate, and build job history is written atomically to `.sil/jobs.json`. Jobs still running when the TUI exits become stale on the next start and can be retried; no daemon resumes half-written work.
8. **KD-14–KD-15, visible concurrency**: The TUI watches relevant file mtimes and shows reload/keep/diff conflict state. `.sil/workspace.lock` records holder, operation, and PID; dead PIDs are stale and live competing holders produce a banner and mutation confirmation.
9. **KD-16–KD-17, doctor**: `sil project doctor` reports fix hints, and `--repair-db` backs up a corrupt SQLite database and rebuilds it by best-effort parsing of on-disk sources. `--fix` remains bibliography repair, and `sources/` is not deleted.
10. **KD-18–KD-19, onboarding**: Starting the TUI without a project opens a wizard for recent projects, a path, initialization, or doctor. `sil init --demo` creates a small offline synthetic project, not a copyrighted paper.
11. **KD-20–KD-23, writing and agent handshake**: The TUI can open the latest estimate report, run a non-blocking draft build and jump to its first reported LaTeX error, show ranked grounding results without inserting citations, and show selected uncommitted draft/bibliography changes plus a Sci-Action proposal. These are review surfaces only.
12. **KD-24–KD-27, boundaries**: Never auto-commit. Do not add a GUI, daemon, sixth tab, split-pane source/draft layout, hard `flock`, multi-query digest watch list, experiment dashboard, Releases/prebuilt distribution, or new MCP tools. MCP remains six workflow-oriented tools.

## Explicit Reversals
Stage 14 intentionally closes selected residuals from ADR-013 and ADR-015 without changing their broader durability guarantees:

- ADR-015's TUI fetch path changed from `parse=false` to composite fetch+parse; auto-open remains off.
- ADR-015's no-section-picker residual is closed by the note and cite section pickers.
- ADR-015's no-visible-triage residual is closed only with derived badges; stored triage states remain rejected.
- ADR-013/ADR-015's silent advisory-lock behavior is now visible with PID liveness and confirmation, but the filesystem lock remains advisory.
- ADR-013's database-repair residual is closed by `sil project doctor --repair-db`; ordinary `--fix` semantics are unchanged.
- The prior TUI estimate residual is closed with an estimate report viewer; the estimate remains an offline L0 heuristic, not peer-review truth.

## Residuals
- Split-pane source+draft was not added.
- There are no GitHub Releases or prebuilt binaries.
- The lock is not `flock` and is not NFS-safe.
- Digest has a single effective query and refreshes only during the TUI lifetime.
- There is no experiment or `data/` dashboard.
- Search/rank surface drift from Stage 12 remains.
- The embed-cache primary key is still `content_hash`.
- Windows atomic rename behavior is unproven.
- W4 is an uncommitted diff/proposal view only, not a commit or patch application workflow.
- Auto-open of the reader remains off.
