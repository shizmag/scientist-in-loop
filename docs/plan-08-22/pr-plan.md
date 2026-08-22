# Stage 16 / Wave 08-22 - Agent-ready scientific workspace

**Status:** Design plan for implementation dispatch  
**On execute:** Implement the PRs and verification gates described here. This document alone changes no product behavior.

| Field | Value |
|---|---|
| **Project** | scientist-in-loop (`sil`) |
| **Date** | 2026-08-22 |
| **Baseline** | Stage 15 shipped: deterministic checks, discovery, package/skill locks, production MCP, and five-tab TUI |
| **Predecessor** | `docs/plan-08-15/`, ADR-018 through ADR-020, `STAGES.md` |
| **Target path** | `docs/plan-08-22/` |
| **Theme** | Make AI agents reliably useful through deterministic context, skills, MCP contracts, and verification |
| **Primary outcome** | An agent can inspect the workspace, select an appropriate skill, perform one explicit action, verify it, and understand the next safe action without guessing from prose |

---

## 1. Executive Summary

`sil` already has the right product boundary: a structured scientific workspace with six workflow-oriented MCP tools, locked skill packs, atomic writes, SQLite memory, deterministic manuscript checks, and never-auto-commit governance. The remaining weakness is not a missing large feature. It is contract drift and agent ambiguity between context, skills, MCP actions, CLI behavior, and verification.

Stage 16 makes the existing surfaces agent-ready:

1. **Context becomes a deterministic state contract.** The agent receives a compact, structured snapshot of what exists, what is invalid, what changed, what actions are available, and which skills were selected.
2. **Skill selection becomes declarative.** Skills advertise task coverage, required inputs, capabilities, conflicts, and output expectations instead of relying mainly on substring matching.
3. **MCP actions become explicit workflows.** Each action validates inputs, reports preconditions, describes affected paths, supports dry-run where safe, returns stable result shapes, and exposes actionable error codes.
4. **The agent loop becomes inspectable.** Every mutation follows `context -> inspect -> propose -> execute -> verify -> report`.
5. **CLI, TUI, MCP, and skills share fixtures.** The same project state must produce equivalent facts and policy decisions across all surfaces.

The result is not an autonomous researcher. It is a dependable operator interface that helps an agent make fewer assumptions and recover cleanly when a user decision, network provider, toolchain, or workspace state blocks progress.

### Product promise

For a valid project, an agent should be able to ask:

> What is the current state, what can I safely do next, what will change, and how do I know it worked?

`sil` should answer all four questions with structured data and human-readable explanations.

---

## 2. Feedback Converted Into Constraints

| Requirement | Stage 16 interpretation |
|---|---|
| Reduce overhead | Prefer deterministic contracts and reuse existing use cases; do not introduce a new product or large domain model |
| Make skills more useful | Skills declare applicability and expected workflow, not just Markdown instructions |
| Make MCP more useful | Preserve the six tool names; improve action schemas and result semantics rather than multiplying tools |
| Keep humans in control | No auto-commit, no hidden mutation, no generic shell execution, no silent citation insertion |
| Make automation dependable | Validate preconditions before mutation, make safe operations idempotent, verify after mutation |
| Keep it testable | Offline fixtures, fake providers, fake compiler, stable fingerprints, and cross-surface assertions |
| Keep AI optional | Core state and checks remain deterministic; an LLM may interpret or plan but does not define truth |
| Avoid migration pain | Existing tool names and action names remain; additive result fields are preferred where compatibility matters |

---

## 3. Goals and Non-Goals

### Goals

1. Define a versioned `AgentState`/context contract shared by context generation and MCP orientation.
2. Include stable project facts: root identity, source/index health, manuscript/check status, structure, TODOs, bibliography, available skills, capabilities, pending jobs, and safe next actions.
3. Make context compact by default, with explicit expansion flags for large draft/source content.
4. Explain skill selection with stable reasons and deterministic ordering.
5. Add declarative skill metadata for tasks, inputs, capabilities, conflicts, and output expectations.
6. Add a common MCP action envelope with operation identity, dry-run/precondition information, changed paths, verification status, and next actions.
7. Standardize user-facing and machine-facing error codes across MCP actions.
8. Make repeated safe requests predictable and prevent accidental duplicate mutations.
9. Expose enough information for an agent to perform one bounded workflow step at a time.
10. Verify parity between CLI, MCP, TUI models, context, and skills using shared offline fixtures.
11. Document subagent roles and provide agent-facing skill instructions that encourage inspect/verify behavior.

### Non-goals

- No new top-level MCP tools; the six shipped workflow tools remain the compatibility surface.
- No generic `exec`, shell, filesystem traversal, or arbitrary code execution tool.
- No autonomous Git commit, push, merge, reset, checkout, or destructive rollback.
- No background daemon, scheduler, cron integration, or permanent agent process.
- No external experiment runner, GPU scheduler, container manager, DVC/MLflow/W&B integration, or result-quality judgment.
- No implicit network access from context generation or deterministic checks.
- No automatic citation insertion solely because discovery found a candidate.
- No hidden prestige score or claim that an AI review is scientific truth.
- No second GUI and no sixth TUI tab.
- No replacement of SQLite, `structure.yaml`, `paper_draft.tex`, Git, template locks, or skill locks as the project contract.
- No broad rewrite of all existing skill content before the registry and contract are proven.

---

## 4. Target Agent Workflow

Every agent-facing operation should fit this bounded loop:

```text
1. context
   Read the current AgentState and selected skill explanations.
2. inspect
   Use read/search/get actions to gather only the evidence required.
3. propose
   Describe the intended action, preconditions, affected paths, and risk.
4. execute
   Invoke exactly one bounded MCP action or an explicitly documented action batch.
5. verify
   Run the relevant deterministic check or postcondition verification.
6. report
   Return changed paths, proposals, warnings, verification, and the next safe action.
```

The server does not need to orchestrate an entire autonomous loop. It must make each step sufficiently explicit that an agent can orchestrate it safely.

### Agent state machine

The state is classified using stable values:

| State | Meaning |
|---|---|
| `ready` | Required inputs exist and at least one safe action is available |
| `needs_input` | A human choice or missing argument is required |
| `blocked` | A precondition, invariant, lock, or capability prevents the action |
| `changed` | A mutation completed but verification is still pending |
| `verified` | The requested operation completed and its postcondition passed |
| `failed` | The operation did not complete; details and retry guidance are present |
| `stale` | The state was computed before a relevant file/job/config change |

These are workflow states, not scientific conclusions.

---

## 5. Architecture

### 5.1 Existing layers to preserve

```text
sil CLI
sil TUI                 surface adapters
sil MCP  ----------------------+
                                  |
                              sil-app
                                  |
     sil-agent ------------- sil-core contracts
        |                         |
  skill registry              checks / config / paths / errors
                                  |
          sil-db / sil-parse / sil-latex / sil-api / sil-git
```

The new contract types belong in a low-level crate only when they are genuinely shared. Composite workflow assembly belongs in `sil-app`; MCP serialization belongs in `sil-mcp`; skill metadata and selection belong in `sil-agent`; presentation remains in CLI/TUI adapters.

### 5.2 Proposed contract types

Names are guidance, not an excuse to create duplicate representations.

```rust
pub struct AgentState {
    pub schema_version: String,
    pub state: AgentStateKind,
    pub project: ProjectIdentity,
    pub inputs: InputSnapshot,
    pub health: HealthSummary,
    pub structure: StructureSummary,
    pub work_items: Vec<WorkItemSummary>,
    pub literature: LiteratureSummary,
    pub skills: SkillSelectionSummary,
    pub capabilities: CapabilitySummary,
    pub jobs: JobSummary,
    pub actions: Vec<AvailableAction>,
    pub warnings: Vec<AgentFinding>,
}
```

The exact types must avoid embedding full manuscripts, full source text, or volatile timestamps in the stable fingerprint. Each item should carry a stable ID, concise summary, canonical path where applicable, and an optional expansion route.

### 5.3 Context fingerprint

The state contains:

- `schema_version`
- normalized project-relative paths
- content/config/check fingerprints already used by the project
- checker and skill-registry versions
- selected skill IDs and versions
- stable summaries of findings and available actions

It excludes:

- current wall-clock time
- provider latency
- temporary job IDs unless explicitly requested
- local absolute paths
- raw secrets or environment values
- arbitrary model output

Identical normalized inputs must produce the same stable fingerprint. A separate volatile execution section may report retrieval time and job IDs.

### 5.4 MCP action envelope

All workflow actions should converge on a result shape containing:

```text
operation_id
schema_version
status
summary
preconditions
changed_paths
created_or_updated_ids
verification
warnings
next_actions
commit_proposal
```

Read-only actions may omit mutation fields but should retain status, evidence, warnings, and next actions. Existing action-specific fields remain available under a typed payload or additive fields during migration.

### 5.5 Preconditions and postconditions

Each mutating action declares:

- required project root and path scope
- required source/reference/draft existence
- whether a lock or conflict banner blocks mutation
- whether network or optional tooling is required
- whether the action is idempotent
- which postcondition is verified

The server checks preconditions before calling the existing use-case function. Verification must inspect durable state after the write, not infer success from a returned string.

### 5.6 Dry-run policy

Dry-run is required for actions that can describe a mutation without external side effects, including:

- bibliography upsert/promote
- draft TODO or structure updates
- skill install/update/remove planning
- proposal generation
- source parse planning when no network fetch occurs

Dry-run is not a fake network call or fake compilation. For fetch/build actions it reports the planned dependencies and required capabilities, then clearly states that execution is not simulated.

---

## 6. PR Breakdown

Each PR has one conceptual owner, a bounded file surface, an acceptance gate, and a safe commit point. Implementers must not combine PRs merely because they touch adjacent code.

### PR-A: Agent state and deterministic context contract

**Purpose:** Replace loosely assembled context with a compact, versioned, explainable state snapshot.

**Likely surface:** `sil-core` shared contract types, `sil-agent/src/context.rs`, `sil-app` context use case, context CLI, MCP `sil_context`, tests and fixtures.

**Work:**

- Define `AgentState`, `AgentStateKind`, summaries, findings, available actions, and schema version.
- Separate stable state from volatile execution metadata.
- Add compact/default and explicit expansion modes.
- Include input fingerprint and source-of-truth paths.
- Include deterministic action availability with reason codes.
- Preserve existing context flags and skill selection compatibility.
- Redact secrets and avoid leaking absolute paths in serialized context.
- Add JSON schema or equivalent contract fixture.

**Acceptance:** The same fixture produces byte-equivalent stable state through CLI context and MCP context, apart from explicitly volatile fields.

**Commit checkpoint:** Commit contract types and fixture first; then implementation; then surface adapters and docs. Do not proceed to skill routing until the fixture schema is reviewed.

### PR-B: Declarative skill metadata and deterministic routing

**Purpose:** Make skill selection predictable, inspectable, and useful for task planning.

**Likely surface:** `sil-agent/src/skills.rs`, `registry.rs`, pack manifests under `templates/agent` and `sil-agent/packs`, CLI skill commands, MCP context skill reporting, tests.

**Skill metadata:**

- stable skill ID and version
- task tags/intents
- required files or project capabilities
- optional files
- provided capabilities
- conflicting skills
- priority/order
- expected inputs
- expected outputs
- verification command or MCP action ID
- license/provenance already required by pack registry

**Work:**

- Keep substring matching only as a compatibility fallback.
- Add deterministic scoring and tie-breaking.
- Return selection reasons, rejected candidates, missing requirements, and conflicts.
- Distinguish `selected`, `available`, `missing`, `incompatible`, and `unsupported`.
- Ensure `SYSTEM.md` remains mandatory.
- Add skill-specific context budgets and explicit expansion instructions.
- Update core skills to state inspect/modify/verify behavior.

**Acceptance:** Equivalent tasks select the same skill set regardless of map/order iteration; ambiguous tasks report ambiguity instead of silently selecting an arbitrary pack.

**Commit checkpoint:** Commit metadata schema and registry validation before migrating packs. Commit each migrated first-party pack separately where practical.

### PR-C: MCP action contracts and structured errors

**Purpose:** Make the existing six tools predictable to an AI agent without expanding the tool count.

**Likely surface:** `sil-mcp/src/protocol.rs`, `sdk.rs`, `tools/mod.rs`, `security.rs`, `sil-core/src/error.rs`, `sil-app` action helpers, MCP tests.

**Work:**

- Define per-action typed input validation.
- Add common result envelope and stable schema version.
- Add error code taxonomy, for example:
  - `invalid_input`
  - `not_in_project`
  - `missing_input`
  - `precondition_failed`
  - `conflict_detected`
  - `capability_unavailable`
  - `provider_unavailable`
  - `not_found`
  - `already_applied`
  - `verification_failed`
  - `internal_failure`
- Return affected paths and commit proposals consistently.
- Add `dry_run` to appropriate mutation actions.
- Add idempotency keys or deterministic duplicate detection where the existing domain permits it.
- Return `next_actions` with action ID, reason, and required input shape.
- Preserve explicit-root security and six tool names.
- Ensure errors never expose stack traces, secrets, or arbitrary host paths in normal output.

**Acceptance:** An agent can distinguish invalid input, blocked state, provider failure, successful mutation, and successful mutation with failed verification without parsing human prose.

**Commit checkpoint:** Commit error taxonomy and envelope tests before changing individual handlers. Migrate one read action and one mutation action as reference implementations before migrating the remaining actions.

### PR-D: Bounded workflow helpers and postcondition verification

**Purpose:** Make common agent tasks complete in small, verifiable steps.

**Candidate workflows:**

- source triage: inspect source -> parse -> inspect metadata -> propose citation
- bibliography cleanup: inspect draft entries -> validate -> dry-run repair -> apply -> verify
- manuscript TODO: list TODO -> inspect section -> update one TODO -> run check -> report
- pre-submission: context -> check -> build/doctor -> report blockers
- skill-assisted review: select review skill -> load bounded context -> write review report -> verify report path

**Likely surface:** `sil-app`, existing use-case functions, `sil-core` action/precondition types, MCP handlers, CLI adapters, tests.

**Work:**

- Add reusable precondition and postcondition helpers rather than duplicating checks in MCP handlers.
- Ensure every mutation has a durable verification path.
- Make repeated calls safe or return `already_applied` with evidence.
- Keep one mutation per action unless an existing action is explicitly composite, such as fetch+parse.
- Add bounded result sizes and pagination/continuation hints for large literature/draft results.

**Acceptance:** Each workflow has a fixture-driven happy path, blocked path, repeated-call path, and recovery path.

**Commit checkpoint:** Commit each workflow helper only after its reference MCP action and CLI path are tested. Do not add “smart” batching without a clear idempotency contract.

### PR-V: Cross-surface parity and agent verification suite

**Purpose:** Prove that the same workspace facts and policies hold across CLI, TUI models, MCP, and skills.

**Likely surface:** `tests/fixtures/pr-v/` or a new Stage 16 fixture, e2e tests, MCP tests, TUI model tests, golden JSON, verification report.

**Work:**

- Add a canonical fixture with:
  - valid and invalid manuscript inputs
  - one parsed and one unparsed source
  - draft and promoted bibliography entries
  - open and resolved TODOs
  - a skill lock with one compatible and one unsupported pack
  - pending/stale job data
  - a conflict/lock scenario
- Compare normalized facts, not presentation strings.
- Assert action availability and error-code parity.
- Assert stable fingerprints across repeated runs.
- Add mutation replay tests.
- Add fake provider and fake compiler tests for failure honesty.

**Acceptance:** The verification report records pass/fail for every contract and documents residual gaps. Stage 16 is not complete until the ordinary workspace gates pass.

**Commit checkpoint:** Commit fixture and normalized comparison helpers first; then each parity family; finally update `STAGES.md` and this plan’s verification report.

### PR-Z: Documentation and migration

**Purpose:** Make the new contract usable by humans and agents.

**Work:**

- Update README MCP and skills sections.
- Document context schema/version and result envelope.
- Add examples for inspect/propose/execute/verify.
- Document stable error codes and next-action semantics.
- Update first-party skills with explicit input/output/verification sections.
- Add migration notes for clients that currently parse action-specific prose.
- Add ADR for the agent contract if the shared state/result envelope becomes a durable architectural boundary.

**Acceptance:** A new agent integration can use the documented fixture and schema without reading implementation details.

---

## 7. Execution Waves

```text
Wave 0: A contract types + fixture | C error/envelope design
Wave 1: A context assembly       | B skill metadata/registry
Wave 2: B first-party pack migration | C reference action migrations
Wave 3: D workflow helpers       | C remaining action migrations
Wave 4: V fixture parity          | Z documentation preparation
Wave 5: V full gates              | Z final docs, STAGES, ADR
```

Dependencies:

- B depends on the state contract’s skill summary shape but may develop registry metadata in parallel.
- C depends on stable error and result types, but can prototype against fixture types.
- D depends on C’s precondition/result semantics and existing `sil-app` use cases.
- V depends on A, B, C, and D.
- Z is final except for docs needed to validate public schemas during implementation.

If scope must slip, keep in this order:

1. Stable context fingerprint and action availability.
2. Error taxonomy and common MCP result envelope.
3. Declarative skill selection with reasons.
4. Parity fixture and mutation verification.
5. Workflow convenience helpers.
6. Additional skill content and polish.

---

## 8. Role of Subagents

Subagents are implementation assistants, not autonomous authorities. They must work inside the task’s file boundary, report evidence, and never commit unless the supervising human explicitly requests a commit.

### Planning/research subagent

**Role:** Inspect existing symbols, callers, fixtures, and docs before an implementation starts.

**Deliverables:** Relevant files, current contracts, dependency risks, proposed tests, and unresolved questions. No code changes.

### Contract subagent

**Role:** Implement or review shared types, schemas, serialization, and compatibility behavior.

**Restrictions:** No changes to action business logic unless required to compile. Must add round-trip and malformed-input tests.

### Skill subagent

**Role:** Migrate skill manifests and selection logic, then update one bounded skill pack at a time.

**Restrictions:** Must preserve license/provenance and managed/local separation. Must not silently overwrite local skills or broaden permissions.

### MCP subagent

**Role:** Migrate one action family and its schemas/results/errors.

**Restrictions:** Must preserve tool names, root confinement, no-shell policy, and never-auto-commit behavior. Must include invalid, blocked, success, and repeated-call tests.

### Verification subagent

**Role:** Build fixtures, run focused tests, compare normalized CLI/TUI/MCP output, and review failure honesty.

**Restrictions:** Does not waive failures because an implementation “looks right.” Reports exact command, result, and residual gap.

### Review subagent

**Role:** Review for regressions, compatibility drift, security leaks, nondeterminism, and missing tests.

**Required questions:**

- Can an agent distinguish failure classes without parsing prose?
- Is the action safe to repeat?
- Are affected paths and verification evidence truthful?
- Does any path escape the configured root?
- Could a hidden network/tool invocation make output nondeterministic?
- Did the change accidentally add an autonomous commit or execution capability?

### Human decision points

Human review is required before:

- changing a public context/result schema version
- changing a tool or action name
- adding a mutation or capability to a skill
- changing path allowlists or installer behavior
- adding a composite action with multiple mutations
- accepting a new external dependency
- declaring Stage 16 shipped

---

## 9. Commit Strategy

The project’s existing “never auto-commit” rule applies to both product behavior and implementation workflow. A plan implementer may create local commits only when explicitly asked; the application itself must continue returning proposals rather than committing.

### Implementation commits

Use small, reviewable commits with one purpose:

1. Contract types and schema fixtures.
2. Context assembly and stable fingerprint.
3. Skill metadata and registry validation.
4. First-party skill migration.
5. MCP envelope/error taxonomy.
6. Reference MCP action migration.
7. Remaining action migrations.
8. Workflow postconditions and idempotency.
9. Cross-surface fixture and parity tests.
10. Documentation, ADR, and Stage 16 status.

Each commit should compile and pass its focused tests. Avoid a giant final formatting or migration commit that obscures behavior changes.

### Product commit proposals

Agent-facing mutation results must continue to include `Sci-Action` proposals where applicable. The common envelope should identify:

- proposal action
- subject/message
- changed paths
- whether the mutation was actually applied
- whether verification passed

`dry_run=true` must never produce a claim that a file changed.

### Safe stopping moments

Stop and review after:

- schema fixture is stable
- context parity passes
- skill selection reasons are deterministic
- reference MCP action passes all error classes
- every migrated mutation has a postcondition test
- full parity fixture passes

At each stopping moment, the workspace should be buildable and tests should describe the current behavior accurately.

---

## 10. Verification Strategy

Verification is layered. No single end-to-end test is sufficient because the risk is contract drift across surfaces.

### 10.1 Unit tests

Test pure logic for:

- state normalization and stable fingerprinting
- path normalization and secret redaction
- skill metadata validation
- deterministic skill scoring/tie-breaking
- conflict and capability resolution
- error-code mapping
- action availability and next-action generation
- idempotency classification
- result-envelope serialization
- pagination and bounded output

### 10.2 Contract tests

For every public schema:

- serialize a valid value and round-trip it
- reject missing required fields
- reject wrong types and unknown unsafe values where appropriate
- preserve additive compatibility fields
- assert stable schema version
- assert no absolute host paths or secrets leak

### 10.3 MCP tests

For each of the six tools and each action family, cover:

- valid request
- missing required argument
- invalid enum/action
- path outside project root
- missing project file
- unavailable optional capability
- provider/network failure fixture
- lock/conflict precondition
- successful read
- successful mutation
- dry-run mutation
- repeated mutation
- postcondition failure
- malformed or oversized result input

Also test that the read loop remains responsive for bounded operations and that cancellation/timeout behavior is honestly reported where supported by the negotiated protocol.

### 10.4 CLI/TUI/MCP parity

Use one fixture and compare normalized representations:

| Fact | CLI | TUI | MCP |
|---|---:|---:|---:|
| Project identity | yes | model | yes |
| Manuscript/check findings | yes | model/render input | yes |
| TODO summaries | yes | model | yes |
| Source parse status | yes | model | yes |
| Bibliography status | yes | model | yes |
| Selected skills/reasons | yes | context/model | yes |
| Available actions | yes | command registry | yes |
| Error classification | exit/report | `UserError` | code/result |
| Changed paths | proposal/report | status/history | result |
| Verification status | report | job/result | result |

Presentation text may differ. Facts, IDs, severity, action availability, and policy decisions may not.

### 10.5 Replay and idempotency tests

For every supported mutation:

1. Run it once and capture the result.
2. Run the same request again.
3. Confirm no duplicate BibTeX entry, TODO, note, file, or lock record is created.
4. Confirm the second result says `already_applied` or equivalent and includes evidence.
5. Change a precondition and confirm the operation is blocked rather than silently reinterpreted.

### 10.6 Failure-injection tests

Use existing atomic-write, SQLite, provider, package, and compiler fixtures plus new cases for:

- file changed between inspect and execute
- active workspace lock
- dead lock PID cleanup
- interrupted job
- corrupt skill lock
- unsupported host capability
- malformed provider response
- partial provider result
- failed bibliography verification after write
- stale generated PDF
- output-size limit

The key assertion is failure honesty: no result may claim `verified` when only the mutation call returned successfully.

### 10.7 Agent scenario tests

Each scenario is a scripted sequence of structured requests, not an LLM benchmark:

1. **Orient empty project:** context reports missing inputs and next actions.
2. **Parse one source:** inspect, execute parse, verify indexed status.
3. **Triage a source:** inspect metadata, dry-run BibTeX upsert, apply, verify citation key.
4. **Resolve one TODO:** list, inspect draft section, update, run check, report result.
5. **Review manuscript:** select review skill, load bounded context, write report, verify report file.
6. **Recover from conflict:** inspect reports conflict, refuses mutation, reloads, retries after user resolution.
7. **Provider outage:** discovery returns partial/failure status and does not invent candidates.
8. **Repeated request:** second request is idempotent and does not duplicate data.
9. **Unsupported skill host:** capability report is partial/unsupported and suggests alternatives.
10. **No network mode:** deterministic context and local checks still work.

### 10.8 Required gates

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Focused gates should include the relevant `sil-core`, `sil-agent`, `sil-app`, `sil-mcp`, `sil-tui`, and e2e tests. The final verification report must include command output, fixture names, and residual gaps.

---

## 11. Security, Safety, and Trust Rules

1. MCP paths remain confined to explicit project/package roots.
2. Skills cannot gain new permissions merely by being selected.
3. Skill metadata is descriptive; capability checks remain enforced by the host.
4. Network access is explicit in action metadata and results.
5. No skill can cause an automatic Git commit.
6. No result may claim scientific validity from heuristic review or estimate output.
7. User-authored local skills are never overwritten by managed updates.
8. Error details are useful but do not disclose secrets, full environment dumps, or host-specific sensitive paths.
9. Dry-run output must clearly separate planned, simulated, and verified facts.
10. A stale context fingerprint must be detectable before a mutation proceeds.
11. Long-running work remains bounded, cancellable where supported, and persisted through the existing job mechanism.
12. The agent must be able to stop after any action without leaving an unreported half-state.

---

## 12. Documentation Deliverables

- `README.md`: updated MCP/skills workflow and one complete inspect-to-verify example.
- `STAGES.md`: Stage 16 scope, shipped guarantees, and residual limitations.
- ADR for the shared agent contract if the state/result envelope is accepted as a long-lived boundary.
- Schema examples under `docs/plan-08-22/fixtures/` or the established test-fixture location.
- Skill author guide covering metadata, inputs, outputs, permissions, and verification.
- MCP client migration guide covering error codes, result envelopes, dry-run, and next actions.
- Verification report listing all commands and residual gaps.

Documentation must say that the agent is an operator over deterministic project capabilities, not an autonomous scientific authority.

---

## 13. Success Criteria

Stage 16 is complete only when all are true:

1. Context has a documented schema version and stable fingerprint.
2. CLI and MCP context expose equivalent normalized facts.
3. Skill selection is deterministic and explains its decision.
4. All six MCP tools retain their names and root-security behavior.
5. Migrated actions return stable error codes and structured next actions.
6. Mutations report affected paths, proposals, and postcondition verification.
7. Safe repeated calls do not duplicate durable records.
8. At least five end-to-end agent scenarios pass offline.
9. Cross-surface parity tests pass on the shared fixture.
10. `cargo test --workspace`, Clippy, and formatting pass.
11. No new generic execution, daemon, auto-commit, or hidden network capability is introduced.
12. Verification and documentation record known residuals honestly.

### Residuals that may remain

- Provider-specific metadata differences and live API freshness.
- Host-specific MCP installation hooks.
- Full external skill capability on unsupported hosts.
- Compiler/toolchain differences during real builds.
- Human ambiguity where multiple skills or actions are equally applicable.

These must be represented as explicit statuses or warnings, never silently hidden.
