# Stage 15 implementation prompts

Run one implementation agent per PR. Every agent must read `../pr-plan.md`, inspect current code before editing, preserve unrelated worktree changes, and stop at its assigned ownership boundary.

## Dispatch order

```text
Wave 0: A1 | B1 | C1 | D1
Gate V1
Wave 1: A2 | B2 | C2
Gate V2
Wave 2: A3 | B3 | E1
Gate V3
Wave 3: B4 | C3 | D2
Gate V4
Wave 4: A4 | B5 | D3 | E2
Gate V5
Wave 5: B6
Gate V6
Wave 6: V
Gate V7
Wave 7: Z
Gate V8
```

Parallel means semantic independence in isolated worktrees, not zero textual overlap. The wave integrator merges PRs sequentially using Section 9.1 file reservations in the main plan. If an earlier PR changes a shared API, later agents adapt to the landed API rather than restoring the plan's illustrative sketch verbatim.

## Shared rules

1. Never auto-commit.
2. No required test may use the public network.
3. Use `sil-app` for policy shared by more than one surface.
4. Use atomic writes for every durable file.
5. Canonicalize and confine caller/package paths before access.
6. Preserve raw provider and venue values alongside normalized data.
7. Do not silently resolve ambiguity or merge uncertain works.
8. Do not make scientific-result changes, word-count changes, hashes, or estimate-score changes fail `sil check`.
9. Default human output is compact; structured JSON remains complete.
10. Keep the six existing MCP tool names.
11. Do not vendor ARS or redistribute third-party template files without verified permission.
12. External experiment execution and symlink management are out of scope.

## Prompt index

| PR | Prompt |
|----|--------|
| A1 | [Check contract](PR-A1-check-contract.md) |
| A2 | [Manuscript graph](PR-A2-manuscript-graph.md) |
| A3 | [Check use case](PR-A3-check-usecase.md) |
| A4 | [Check surfaces](PR-A4-check-surfaces.md) |
| B1 | [Venue catalogue](PR-B1-venue-catalogue.md) |
| B2 | [Discovery schema](PR-B2-discovery-schema.md) |
| B3 | [Crossref/OpenAlex](PR-B3-crossref-openalex.md) |
| B4 | [OpenReview/DBLP](PR-B4-openreview-dblp.md) |
| B5 | [Candidate use case](PR-B5-candidate-usecase.md) |
| B6 | [Discovery surfaces](PR-B6-discovery-surfaces.md) |
| C1 | [Package foundation](PR-C1-package-foundation.md) |
| C2 | [Template packs](PR-C2-template-packs.md) |
| C3 | [Submission release](PR-C3-submission-release.md) |
| D1 | [MCP root/security](PR-D1-mcp-root-security.md) |
| D2 | [MCP SDK](PR-D2-mcp-sdk.md) |
| D3 | [MCP installers](PR-D3-mcp-installers.md) |
| E1 | [Skill registry](PR-E1-skill-registry.md) |
| E2 | [External skill packs](PR-E2-external-skill-packs.md) |
| V | [Verification](PR-V-verification.md) |
| Z | [Docs](PR-Z-docs.md) |

## Deliverable format

Each agent returns:

- changed files and behavior;
- tests run and exact outcomes;
- any plan deviation and why;
- remaining risks or follow-up limited to its PR;
- confirmation that no commit was created.
