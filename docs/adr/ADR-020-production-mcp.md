# ADR-020: Production MCP Boundary

## Status

Accepted for Stage 15; ship status is gated by `docs/plan-08-15/verification-report.md`.

## Decision

The MCP server uses the Rust SDK implementation and binds to an explicit,
canonical project root for installed clients. Direct interactive CWD discovery
remains a compatibility fallback and is reported. Tools are typed and
structured; project data is exposed through resources, workflow guidance
through prompts, and mutations through tools. The six workflow tool names
remain the compatibility surface.

Installer adapters own only the `scientist-in-loop` entry, preserve unknown
configuration fields, fail closed on malformed JSON, back up before atomic
writes, and expose status/uninstall. Hooks are optional host capabilities;
unsupported hooks report unsupported rather than claiming success. MCP does
not provide generic shell execution.
