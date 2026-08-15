# PR-E1 - Skill registry, managed/local split, and explicit update

## Role

Skill registry engineer. Own generic skill-pack lifecycle and routing.

## Goal

Replace four hard-coded Markdown booleans with validated `skill-pack.yaml` entrypoints, `.sil/skills.lock`, managed package projections, local user skills, explicit update/diff/rollback, and host capabilities.

## Requirements

1. Implement the skill manifest contract from plan Section 6.5 on C1 package primitives.
2. Registry supports arbitrary validated entrypoints, nested declared support files/resources, trigger metadata, compatibility, capabilities, source/license/digests.
3. Separate immutable managed projections from user-authored local skills. Define paths without inventing undocumented top-level project directories.
4. `sil init --update` must not overwrite changed skill content. Add a safe migration/backup from existing built-ins.
5. Add explicit list/show/install/verify/check-update/diff/approve-update/remove/rollback use cases and CLI adapters.
6. Dirty managed files block update; local files survive all managed updates.
7. MCP prompt/resource exposure will consume the same registry; keep host-neutral APIs.
8. Capabilities distinguish read/write/network/process and full/partial/unsupported host requirements.

## Tests

Routing arbitrary entrypoints, nested resources, traversal rejection via C1, lock/compatibility/license, dirty update refusal, rollback, local preservation, `init --update` migration, deterministic list/order.

## Out of scope

Visualize Article/ARS content, host hooks, MCP SDK wiring beyond registry API, generic process execution.

## Verify

```bash
cargo test -p sil-agent -p sil-app -p sil
cargo clippy -p sil-agent -p sil-app -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Registry layout/migration, commands, tests, no commit.
