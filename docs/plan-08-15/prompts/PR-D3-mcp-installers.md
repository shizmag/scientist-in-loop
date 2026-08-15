# PR-D3 - Safe MCP installers and optional hooks

## Role

Installer engineer. Own host/platform configuration adapters.

## Goal

Replace path guesses/direct writes with tested project-scoped install/status/uninstall adapters that preserve host config and optionally install honest nonblocking hooks.

## Requirements

1. Add typed client/platform adapters for the currently supported clients plus OpenCode. Verify current schemas/paths during implementation and record sources.
2. Install command includes canonical binary and explicit `--project <root>`.
3. Parse existing config fail-closed. Invalid/non-object config is an error; do not replace it.
4. Preserve unknown fields; create timestamped backup; atomically write; mark sil-owned entry sufficiently for safe uninstall.
5. Add `status` and idempotent `uninstall`; remove only sil-owned entries and preserve unrelated servers/settings.
6. Support project/global scope only where the host truly supports it; unsupported combinations return actionable errors.
7. Hooks are optional adapters. A post-write `sil paper check` hook is nonblocking/deduplicated by default. Never claim hook support on an untested host.
8. No installer may copy secrets or machine-specific project paths into tracked project files.

## Tests

Malformed JSON unchanged, unknown fields preserved, backup exists, install twice identical, uninstall ownership-safe, paths with spaces, macOS/Linux/Windows fixtures, OpenCode fixture, unsupported host/platform, hook absent/supported/nonblocking.

## Out of scope

OS package manager/release binaries, generic host plugin framework, blocking scientific warnings, shell profile modification.

## Verify

```bash
cargo test -p sil
cargo clippy -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Supported matrix and sources, backup/atomic behavior, tests, no commit.
