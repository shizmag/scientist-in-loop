# PR-C1 — Reader cite (`b`)

Copy the block below into an agent session (worktree-isolated if parallel with C2). **Depends on A1** only for shared TUI hygiene; may land without A1 if the reader handler is untouched by A1.

---

## Role

You are the **reader-cite engineer** for scientist-in-loop. Ship ONLY PR-C1.

## Goal

From the markdown reader (`InputMode::ReadingSourceMd`), `b` upserts the **current** source into `references.bib` using the same `sil-app` path as Sources-list `b`.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-13/pr-plan.md` §5.4, KD-9, KD-12, KD-14, KD-15
- Reader keys today: `crates/sil-tui/src/app/handlers/mod.rs` `handle_reading_source_md_mode` — scroll + quit + help only
- List `b`: `crates/sil-tui/src/app/bib_actions.rs` + `sil_app::upsert_bib` (`draft: true`)
- Help: `HelpMode::ReadingSourceMd` in `crates/sil-tui/src/app/types.rs` (`keymap_for`)
- `b` is unused in the reader (Sources **list** uses `b`; viewer refs use `c/b/p` in a different mode)

## Shared invariants

1. Minimal diff. Reuse `sil_app::upsert_bib`. Do not fork bib policy.
2. Never auto-commit. Cite-key preserve stays true. Draft marker `% [sil: tui-added]` stays as list `b`.
3. Sci-Action is `UpdateBibliography` if the existing list path already proposes it. No new `SciAction` variant.
4. Esc / `q` still exits the reader. `b` must **not** exit the reader.
5. Prefer unit tests; clippy clean on touched crates.

## Requirements

1. In `handle_reading_source_md_mode`, handle `KeyCode::Char('b')`:
   - Resolve the source at `selected_source_index`.
   - Call the same upsert helper the Sources list uses (extract a shared `append_source_to_bib(source)` on `App` if that removes duplication; do not copy policy).
   - Status line: success / already present / error.
2. Missing project / missing bib file: same error behavior as list `b`.
3. Update `keymap_for(HelpMode::ReadingSourceMd)`: `b` = “Append this source to references.bib”.
4. Tests:
   1. Help overlay / keymap includes `b`.
   2. Handler test or extracted function: given a source with metadata, upsert is invoked (mock or tempfile project). Follow existing TUI test style in `crates/sil-tui/src/app/tests/`.
   3. `q` still leaves `ReadingSourceMd`.

## Out of scope

- Note modal `n` (C2)
- Digest Enter (C3)
- Highlighting / claim grounding / section picker
- Changing Sources-list `b` policy
- Auto-inserting `\cite{...}` into the draft

## Verify

```bash
cargo test -p sil-tui -p sil-app
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Handler change, reused upsert function name, help-overlay line, residual “still no \\cite insert”.
