# PR-E2 - Visualize Article pack and ARS external adapter

## Role

Skill integration engineer. Own two named integrations and their legal/capability boundaries.

## Goal

Ship a pinned optional MIT Visualize Article skill pack and an optional separately licensed Academic Research Skills adapter without vendoring ARS into MIT assets or overstating host support.

## Requirements

1. Package Visualize Article through E1 with exact source revision/release, file hashes, MIT notice, triggers, and read/network capabilities.
2. State accurately that it generates prompts for external image providers, not figures. Require/record external-provider data-flow consent metadata.
3. Implement ARS as an external adapter/installer descriptor over a pinned upstream source/layout. Preserve CC-BY-NC attribution and require explicit acknowledgement before install/enable.
4. Do not copy ARS files into embedded templates, binary assets, or MIT-managed skill content.
5. Detect host capabilities: subagents, hooks, commands, scripts, resources. Report full/partial/unsupported for each entrypoint/host.
6. Keep upstream files authoritative; sil-specific bridge instructions are original, minimal, and clearly separated.
7. Add content/licensing exclusion tests and third-party metadata consumed by PR-Z.

## Tests

Pack lock/hash/license snapshots, trigger routing, external-network capability prompt, ARS acknowledgement required, no ARS content in embedded/template paths, full/partial capability fixtures, unavailable upstream/cache behavior.

## Out of scope

Calling image APIs, reproducing ARS agent orchestration, relicensing ARS, downloading ARS by default, experiment-agent integration.

## Verify

```bash
cargo test -p sil-agent -p sil-app -p sil-mcp -p sil
cargo clippy -p sil-agent -p sil-app -p sil-mcp -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Pinned sources/licenses, capability matrix, content-exclusion proof, tests, no commit.
