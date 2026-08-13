# Autonomous agent prompts — 2026-08-13 Stage 13 / daily command center

Copy-paste ready prompts for **one agent per PR**. Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not ask one agent to do multiple PRs unless serial and you explicitly chain them |
| **Worktree isolation** | Prefer isolated git worktrees when running A1∥B1 or C1∥C2 |
| **Shared preamble** | Every prompt is self-contained — agents must not depend on chat history |
| **Commit policy** | Agent may create a local commit **only if** the user asked; default = leave unstaged/staged summary |
| **Done criteria** | Green tests listed in the prompt + short summary: files, behavior, residual risk |
| **Do not expand scope** | Anything under “Out of scope” is forbidden even if “obvious” |

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors outside PR scope.
2. Never auto-commit. Sci-Action proposals only. Atomic write ≠ git commit.
3. **Same five TUI tabs.** Dashboard layout stays 2×2. No `sil daily` command. No new MCP tool. No JSON orientation dump.
4. Reuse `sil_app::upsert_bib` / `sil_app::fetch_source` / `sil_latex::update_or_insert_idea_block` / existing TUI job chrome. Do not fork those policies.
5. Prefer unit tests co-located; clippy `-D warnings` on touched crates.
6. Empty digest query disables auto-refresh. Refresh interval minimum is **1 hour**. TUI-lifetime only (no cron/daemon).
7. Reader verbs are only `b` (cite) and `n` (note). No highlight layer, no triage states, no section picker.

## Parallel waves

```text
Wave 0 (parallel):  PR-A1 | PR-B1
Wave 1:             PR-B2          (after A1 + B1)
Wave 2 (parallel):  PR-C1 | PR-C2  (after A1; may overlap B2)
Wave 3:             PR-C3          (after B2)
Wave 4:             PR-V then PR-Z
```

## Prompt index

| PR | File | Depends on | Parallel with |
|----|------|------------|---------------|
| **PR-A1** Live dashboard | [PR-A1-live-dashboard.md](PR-A1-live-dashboard.md) | — | B1 |
| **PR-B1** Digest settings | [PR-B1-digest-settings.md](PR-B1-digest-settings.md) | — | A1 |
| **PR-B2** Background digest job | [PR-B2-background-digest.md](PR-B2-background-digest.md) | A1, B1 | C1, C2 |
| **PR-C1** Reader cite (`b`) | [PR-C1-reader-cite.md](PR-C1-reader-cite.md) | A1 | C2, B2 |
| **PR-C2** Reader note (`n`) | [PR-C2-reader-note.md](PR-C2-reader-note.md) | A1 | C1, B2 |
| **PR-C3** Digest Enter → fetch | [PR-C3-digest-open.md](PR-C3-digest-open.md) | B2 | — |
| **PR-V** Verification stage | [PR-V-verify.md](PR-V-verify.md) | A1–C3 | — |
| **PR-Z** Docs / ADR-015 | [PR-Z-docs-adr-015.md](PR-Z-docs-adr-015.md) | V | last |

## Subagent roles

| Role | PR |
|------|-----|
| Dashboard engineer | A1 |
| Settings engineer | B1 |
| Jobs engineer | B2 |
| Reader-cite engineer | C1 |
| Reader-note engineer | C2 |
| Digest-inbox engineer | C3 |
| Verifier | V |
| Docs agent | Z |

## Product defaults (KD)

- Dashboard is the daily view — no `sil daily`
- Agents use existing `sil_context` (6 MCP tools)
- Effective digest query = local if non-empty else global; both empty = off
- `digest_refresh_hours` default 1, min 1
- Refresh when Dashboard is shown and cache is stale; one in-flight job
- Reader `b` → `sil_app::upsert_bib` (`draft: true`)
- Reader `n` → `# -- X -- #` with `from: <filename>`, tag `from-source`
- Digest Enter → existing fetch queue, `parse=false`, no auto-open reader
- Sci-Action: cite = `UpdateBibliography`, note = `EditDraft`. No new variants
