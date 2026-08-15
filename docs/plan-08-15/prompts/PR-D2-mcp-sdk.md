# PR-D2 - Official MCP SDK, typed tools, resources, and prompts

## Role

MCP protocol engineer. Replace transport/lifecycle while preserving product use cases.

## Goal

Migrate the hand-written protocol to the maintained official Rust MCP SDK using D1 parity fixtures, typed per-action inputs/results, project resources, skill prompts, and isolated long-running work.

## Requirements

1. Use the approved official `rmcp` implementation from `modelcontextprotocol/rust-sdk`, target upstream tag `rmcp-v3.1.2`. Verify license, workspace MSRV, stdio, negotiated protocol, cancellation, and progress before editing; pin the exact resolved version/revision in `Cargo.lock`. Stop for a plan amendment only on a demonstrated security/compatibility blocker. Do not substitute another SDK silently or invent a second protocol layer.
2. Preserve the six tool names and all valid shipped actions/results. Additive fields are allowed; breaking changes require explicit migration notes/tests.
3. Use typed request structs and actual validation, including per-action required fields/`oneOf` semantics.
4. Implement protocol initialization/version negotiation, notifications, cancellation, progress, timeouts, and task isolation where supported.
5. Expose read-only resources for project context/manuscript sections/sources/reports and prompts for installed skill entrypoints. Confine roots per D1.
6. Tools delegate to `sil-app`; no CLI shell-out and no generic shell/filesystem tool.
7. Ensure stdout contains protocol only and stderr/logging behavior is client-safe.
8. Keep mutation governance, atomic writes, locks, and `never_committed` responses.

## Tests

D1 parity suite, SDK conformance/in-memory transport, version negotiation, notifications, cancellation/timeout, schema rejection, resource/prompt confinement, six names, HEAD unchanged, no stdout noise.

## Out of scope

Discovery/check actions not yet implemented, client config installation, skill package transport.

## Verify

```bash
cargo test -p sil-mcp -p sil
cargo clippy -p sil-mcp -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

SDK/version rationale, parity result, protocol limitations, no commit.
