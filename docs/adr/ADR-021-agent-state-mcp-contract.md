# ADR-021: Agent State and MCP Structured Action Contract

## Status

Accepted for Stage 16; verified against cross-surface fixture parity.

## Context

Prior to Stage 16, agent interactions with `sil` relied on loosely assembled Markdown context strings, substring-based skill heuristics, and unstructured tool responses. When AI agents interacted via MCP or CLI, they had to parse free-form human prose, and error conditions lacked structured classification. Furthermore, mutating operations lacked explicit dry-run contracts, and skills lacked machine-readable capability requirements and verification rules.

## Decision

1. **Deterministic Agent State (`sil.dev/agent-state/v1`)**:
   - Centralized `AgentState` in `sil-core` containing `project`, `inputs`, `health`, `structure`, `work_items`, `literature`, `skills`, `capabilities`, `jobs`, `actions`, and `warnings`.
   - Built deterministic context hashing (`fingerprint`) that excludes volatile timestamps while capturing normalized input snapshots.
   - Enforced automated secret scrubbing (`redact_secrets`) for API keys and tokens (`SIL_*`, `OPENAI_*`, `ANTHROPIC_*`, `*_SECRET`, `*_TOKEN`) and canonical project-relative path normalization.
   - Formalized high-level state classification (`Ready`, `NeedsInput`, `Blocked`, `Degraded`).

2. **Declarative Skill Routing and Capability Gating**:
   - Migrated all first-party skills (`SYSTEM.md`, `paper.md`, `agent-code.md`, `review.md`, `visualize-article`) to declarative YAML frontmatter with explicit `triggers`, `required_capabilities`, `inputs`, `outputs`, `permissions`, and `verification`.
   - Established deterministic skill routing with score breakdown, capability checks against `HostCapabilities`, lexical tie-breaking, and explicit reason codes for available, selected, and incompatible skills.
   - Standardized structured workflow sections in skill templates: Inspect -> Propose -> Modify -> Verify.

3. **Structured MCP Action Result Envelope (`sil.dev/mcp-result/v1`)**:
   - Standardized all 6 MCP workflow tools (`sil_context`, `sil_sources`, `sil_cite`, `sil_draft`, `sil_review`, `sil_propose`) to return `McpActionResult`.
   - Established a closed `McpErrorCode` taxonomy: `invalid_input`, `not_in_project`, `missing_input`, `precondition_failed`, `conflict_detected`, `capability_unavailable`, `provider_unavailable`, `not_found`, `already_applied`, `verification_failed`, `internal_failure`.
   - Structured precondition verification and durable postcondition inspection on all mutating actions.
   - Added `dry_run: bool` support across mutation actions (`sil_cite`, `sil_draft`, `sil_sources`).
   - Attached structured `next_actions` to guide agent multi-step workflows.

## Consequences & Tradeoffs

- **Parity**: CLI (`sil context --json`) and MCP (`sil_context`) produce byte-equivalent stable state snapshots.
- **Safety**: Secret leakage and arbitrary host path leakage are systematically prevented across all serialized surfaces.
- **Idempotency**: Repeated mutations return predictable statuses (`already_applied` or success) without file corruption.
- **Backward Compatibility**: Human-readable Markdown context remains accessible via default `sil context` CLI invocations, while `--json` and `--envelope` provide machine-readable contracts.
