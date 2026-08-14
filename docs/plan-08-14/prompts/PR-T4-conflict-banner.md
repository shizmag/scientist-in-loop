# PR-T4 — mtime conflict banner

Copy the block below into an agent session. **After T2.**

---

## Role

You are the **watch engineer** for scientist-in-loop. Ship ONLY PR-T4.

## Goal

If `paper_draft.tex`, `references.bib`, or `.sil/config.yaml` changed on disk after the TUI loaded them, and the TUI is dirty, do **not** silently overwrite on Save. Show a banner: Reload / Keep TUI.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.10, KD-14
- Today: `R` reloads. `dirty` flag exists. `pending_external_editor` exists. No mtime snapshot.
- W4 will add “View diff”; T4 may add a stub action “View diff” that says “coming soon” or omit it.

## Shared invariants

1. Minimal diff. No git mergetool. No auto-reload that drops dirty buffers.
2. Never auto-commit.
3. UserError for the banner title if useful (`conflict.disk_newer`).
4. Clippy clean.

## Requirements

1. Remember mtimes (or content hashes if cheaper/reliable) at load / successful save / successful reload for:
   - `paper_draft.tex`
   - `references.bib`
   - `.sil/config.yaml`
2. On tick / focus / `R` / Save: if disk is newer than snapshot **and** `dirty`, set a conflict banner / `InputMode`.
3. Actions:
   - **Reload** — existing reload path; clears dirty if user confirms.
   - **Keep TUI** — dismiss banner; next Save still blocked until they confirm overwrite (explicit “Overwrite disk” confirm).
4. If not dirty and disk is newer: silent or one-line “disk changed — press R to reload” is OK; do not block navigation.
5. Unit tests:
   1. Dirty + newer mtime → save does not write without confirm.
   2. Not dirty + newer mtime → no overwrite of in-memory if user reloads.
   3. After successful save, snapshot updates so a second save is not a false conflict.

## Out of scope

- Full diff widget (W4)
- Workspace lock PID (T6)
- Watching `sources/` PDFs

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Which files are watched, banner mode, save-guard behavior.
