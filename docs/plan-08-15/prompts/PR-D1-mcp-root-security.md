# PR-D1 - MCP explicit root, path security, and parity fixtures

## Role

MCP security engineer. Harden the current server before protocol migration.

## Goal

Make every MCP operation bind to an explicit canonical project root and close caller-controlled skill/path traversal. Capture current valid behavior as parity fixtures for PR-D2.

## Requirements

1. Add `sil project mcp --project <path>` (and compatible direct form) with canonical project validation. Installed/desktop use must not depend on process CWD.
2. Direct interactive invocation may retain CWD discovery only as a reported fallback.
3. Pass an explicit project context/root through tool dispatch instead of repeatedly calling `project_root_from_cwd`.
4. Confine all user-supplied project paths and skill names. Absolute caller paths are accepted only when their canonical target is already under the project or a canonical external root declared by config; they cannot create a new root. Reject other absolute paths, `..`, symlink/package-root escapes, and non-declared skill resources. Skill access uses registry IDs, never arbitrary absolute paths.
5. Ensure lock acquisition/cleanup behavior is not weakened.
6. Add protocol request/response parity fixtures for initialize, notifications, tools/list, each six-tool action family, errors, and shutdown-relevant behavior before SDK migration.
7. Do not add new discovery/check actions yet and do not rewrite the transport in this PR.

## Tests

Launch from HOME with explicit temp project; missing/invalid root; two roots do not cross; configured absolute external root accepted and reported; absolute path outside allowlist rejected; skill traversal/absolute/symlink attacks; current six names and valid result shapes; no stdout contamination in quiet mode.

## Out of scope

SDK migration, resources/prompts, installer config updates, action schema redesign, generic filesystem tool.

## Verify

```bash
cargo test -p sil-mcp -p sil
cargo clippy -p sil-mcp -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Threats closed, parity fixture inventory, tests, no commit.
