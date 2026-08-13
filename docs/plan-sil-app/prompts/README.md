# Autonomous agent prompts — Stage 12 / sil-app use-case layer

Copy-paste ready prompts for **one agent per PR**. Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not ask one agent to do multiple PRs unless serial and you explicitly chain them |
| **Worktree isolation** | Prefer isolated git worktrees when running B1/B2/B3 or C2/C3/C4 in parallel |
| **Shared preamble** | Every prompt is self-contained — agents must not depend on chat history |
| **Commit policy** | Agent may create a local commit **only if** the user asked; default = leave unstaged/staged summary |
| **Done criteria** | Green tests listed in the prompt + short summary: files, behavior, residual risk |
| **Do not expand scope** | Anything under “Out of scope” is forbidden even if “obvious” |

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors outside PR scope.
2. Never auto-commit. Use-cases return `CommitProposal`; adapters never `git commit`.
3. Unify on **richest** behavior (parent KD-3). Role flags only: `draft` (bib), `parse` (fetch).
4. `upsert_bib` always `preserve_cite_key: true` (KD-5). Do not reintroduce a preserve flag on the use-case.
5. Always `write_atomic_str` for `references.bib`.
6. Prefer unit tests co-located; clippy `-D warnings` on touched crates.
7. Do **not** touch TUI hydration apply in `jobs.rs` (KD-13) unless the prompt is explicitly C4 fetch **queue** (still not hydration).
8. Search / rank / estimate / edit-section are **out** of this wave.

## Parallel waves

```text
Wave 0:  PR-A1
Wave 1:  PR-B1 | PR-B2 | PR-B3          (after A1)
Wave 2:  PR-C1                          (after A1; may overlap Wave 1)
Wave 3:  PR-C2 | PR-C3 | PR-C4          (after C1)
Wave 4:  PR-Z                           (after B* and C*)
```

C1 only needs A1 (not B*). It can start as soon as A1 lands.

## Prompt index

| PR | File | Depends on | Parallel with |
|----|------|------------|---------------|
| **PR-A1** sil-app + upsert/promote | [PR-A1-sil-app-bib.md](PR-A1-sil-app-bib.md) | — | — |
| **PR-B1** CLI cite adapters | [PR-B1-cli-cite.md](PR-B1-cli-cite.md) | A1 | B2, B3 |
| **PR-B2** MCP cite adapters | [PR-B2-mcp-cite.md](PR-B2-mcp-cite.md) | A1 | B1, B3 |
| **PR-B3** TUI explicit bib | [PR-B3-tui-bib.md](PR-B3-tui-bib.md) | A1 | B1, B2 |
| **PR-C1** fetch_source use-case | [PR-C1-fetch-usecase.md](PR-C1-fetch-usecase.md) | A1 | B* |
| **PR-C2** CLI fetch adapter | [PR-C2-cli-fetch.md](PR-C2-cli-fetch.md) | C1 | C3, C4 |
| **PR-C3** MCP fetch adapter | [PR-C3-mcp-fetch.md](PR-C3-mcp-fetch.md) | C1 | C2, C4 |
| **PR-C4** TUI fetch job | [PR-C4-tui-fetch.md](PR-C4-tui-fetch.md) | C1 | C2, C3 |
| **PR-Z** STAGES + ADR-014 | [PR-Z-docs-adr-014.md](PR-Z-docs-adr-014.md) | B* + C* | last |

## Product defaults (KD)

- New crate `sil-app` (not a `sil` lib — cycle with TUI/MCP)
- Sync use-cases; no UI / JSON / Ratatui inside `sil-app`
- CLI `cite` stays quiet (no git proposal block)
- Fetch `parse` default true; TUI fetch passes `parse=false`
- Official bib: target DOI/arXiv, then `resolve_official_bibtex_for_source`
- Parse errors on the result, never swallowed
- TUI empty `upsert_parsed("",…)` stays in the TUI adapter
