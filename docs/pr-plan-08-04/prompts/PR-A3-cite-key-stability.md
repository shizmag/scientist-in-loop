# PR-A3 — Cite-key stability on hydrate/upsert

Copy the block below into an agent session. Base on branch containing PR-A1 + PR-A2.

---

## Role

Focused implementer. Ship ONLY PR-A3. Base on branch containing PR-A1 + PR-A2.

## Goal

When upgrading a BibTeX entry with official metadata, preserve the existing cite key so manuscript `\cite{...}` stays valid.

## Repo context

- Local stubs use `slug_cite_key(title|raw)` (`source.rs` `to_bibtex`, `suggest_from_*`).
- Official DOI/arXiv BibTeX often uses publisher keys → current upsert replaces whole block → key churn.
- TUI hydration: `crates/sil-tui/src/app.rs` `poll_background_hydration` marks + upserts official bib.
- CLI: `crates/sil/src/commands/cite.rs`, source fetch also upsert.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI bib add stays non-blocking (local first + background hydrate).
3. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. Add a pure helper, e.g. `rewrite_bib_cite_key(entry: &str, new_key: &str) -> String` or `preserve_cite_key_from(existing_block, new_entry)`.
2. Upsert API options (pick simplest clean design):
   - Preferred: `upsert_bib_entry_with_options(content, new_entry, UpsertOptions { preserve_cite_key: bool })`
   - Default for hydrate/TUI paths: `preserve_cite_key = true` when replacing a match
   - CLI append of brand-new entry: no preserve needed
3. When preserving: keep existing key; still pretty-format fields from official entry; keep completeness rules from A2.
4. Unit tests: stub key `attention_is_all_you_need` + official `@article{Vaswani2017,...}` → result still uses stub key, fields upgraded.
5. Wire TUI hydration success path to use preserve.

## Out of scope

- Rewriting paper_draft.tex cite commands
- Promote/hydrate race (PR-A4) except do not regress marker handling

## Verify

```bash
cargo test -p sil-core
cargo test -p sil-tui --lib
cargo clippy -p sil-core -p sil-tui --all-targets -- -D warnings
```

## Deliverable

API sketch, call sites updated, tests, note any CLI paths still changing keys intentionally.
