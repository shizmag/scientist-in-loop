# Autonomous agent prompts — 2026-08-04 plan

Copy-paste ready prompts for one agent per PR. Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not ask one agent to do multiple PRs unless serial and you explicitly chain them |
| **Worktree isolation** | Prefer isolated git worktrees when running in parallel |
| **Shared preamble** | Every prompt is self-contained — agents must not depend on chat history |
| **Commit policy** | Agent may create a local commit **only if** the user asked; default = leave unstaged/staged summary |
| **Done criteria** | Green tests listed in the prompt + short summary: files, behavior, residual risk |
| **Do not expand scope** | Anything under “Out of scope” is forbidden even if “obvious” |

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI bib add stays non-blocking (local first + background hydrate).
3. Release strip only removes `% [sil: tui-added]` blocks from packages.
4. Do not invent MCP bib write paths or full keybinding remaps.
5. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Parallel waves

```text
Wave 0 (parallel):  PR-A1 | PR-C1 | PR-B1 | PR-C3
Wave 1 (after A1):  PR-A2 | PR-C2
Wave 2 (after A2):  PR-A3
Wave 3 (after A3):  PR-A4
Wave 4 (after A4):  PR-B2
Wave 5 (after B2):  PR-B3  [PR-B4 stretch after B3]
Wave 6 (last):      PR-C4 docs
```

## Prompt index

| PR | File | Depends on | Parallel with |
|----|------|------------|---------------|
| **PR-A1** Pretty BibTeX foundation | [PR-A1-pretty-bibtex.md](PR-A1-pretty-bibtex.md) | — | C1, B1, C3 |
| **PR-A2** Completeness-aware upsert | [PR-A2-upsert-completeness.md](PR-A2-upsert-completeness.md) | A1 | C2 (careful) |
| **PR-A3** Cite-key stability | [PR-A3-cite-key-stability.md](PR-A3-cite-key-stability.md) | A1, A2 | — |
| **PR-A4** Hydration races | [PR-A4-hydration-races.md](PR-A4-hydration-races.md) | A1–A3 | — |
| **PR-B1** Keyboard help | [PR-B1-keyboard-help.md](PR-B1-keyboard-help.md) | — | A1, C1, C3 |
| **PR-B2** Job status chrome | [PR-B2-job-status-chrome.md](PR-B2-job-status-chrome.md) | A4 preferred | — |
| **PR-B3** Sources ingest | [PR-B3-sources-ingest.md](PR-B3-sources-ingest.md) | B2 preferred | — |
| **PR-B4** Sources parse (stretch) | [PR-B4-sources-parse.md](PR-B4-sources-parse.md) | B2, B3 | stretch |
| **PR-C1** Golden negative patterns | [PR-C1-golden-negatives.md](PR-C1-golden-negatives.md) | — | A1, B1, C3 |
| **PR-C2** Resolve reliability | [PR-C2-resolve-reliability.md](PR-C2-resolve-reliability.md) | A1 | A2 |
| **PR-C3** Digest parity | [PR-C3-digest-parity.md](PR-C3-digest-parity.md) | — | A1, B1, C1 |
| **PR-C4** Docs consolidation | [PR-C4-docs.md](PR-C4-docs.md) | all code PRs | last |

## Product defaults

- Cite keys preserved on hydrate
- Completeness preferred over always-replace
- Sources `a`: real fetch preferred, honest stub acceptable fallback
- CLI append does **not** add tui-added by default
