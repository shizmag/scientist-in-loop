# PR-B2 - Discovery/work/candidate SQLite schema

## Role

Discovery DB engineer. Own additive persistence and repository APIs.

## Goal

Add durable local schema for immutable discovery runs/provider records, canonical works/versions/venues, candidates, and append-only candidate events without corrupting current sources/digest data.

## Requirements

1. Add versioned/idempotent migration support suitable for the current unversioned schema.
2. Add logical entities from plan Section 6.3 with foreign keys, indexes, and clear delete policy.
3. Preserve raw provider payload hash/request metadata/cursor/status and venue raw/resolution evidence/catalogue version.
4. Model identifiers and publication versions separately; do not force preprint/conference/journal extensions into one row.
5. Candidate resolution, disposition, and acquisition are orthogonal. Transitions append actor/reason/time and reject invalid state changes.
6. Existing `journal_digest` remains readable during migration. Do not reinterpret title fallback as verified DOI.
7. Fix stale extracted-reference replacement semantics only if required for schema correctness and with focused regression tests; do not mix unrelated DB cleanup.

## Tests

Migration from a current fixture DB, migration rerun, rollback on failure, raw evidence preservation, query/run isolation, candidate events/transition constraints, version relations, catalogue rematch fields, existing DB tests green.

## Out of scope

HTTP, matching/ranking algorithms, CLI/TUI/MCP, live data migration from providers.

## Verify

```bash
cargo test -p sil-db
cargo clippy -p sil-db --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Migration/API summary, compatibility notes, tests, no commit.
