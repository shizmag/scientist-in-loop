# PR-B3 - Provider framework plus Crossref/OpenAlex discovery

## Role

Crossref/OpenAlex provider engineer. Own transport, provider DTOs, fixtures, and adapters only.

## Goal

Introduce injectable offline-testable discovery providers and implement Crossref plus OpenAlex pagination/provenance without candidate merge/rank/UI policy.

## Requirements

1. Add `DiscoveryProvider`, request/page/raw-record types, injectable HTTP transport, provider-specific rate/retry policy, cursor/page handling, cancellation hook, and structured partial errors.
2. Crossref discovery must support proceedings/conference and journal records, cursor pagination, selected venue identifiers/filters where API-supported, and raw request/payload provenance.
3. OpenAlex provides broad discovery/citation-neighborhood metadata and source/venue external IDs. Preserve observed values and retrieval metadata.
4. Never call provider output a canonical venue until B1 resolver evidence is applied by later policy.
5. Respect provider rate limits and `Retry-After`; do not share one global rate bucket across providers.
6. Parse through structured JSON APIs, not ad-hoc string extraction.
7. Required tests use local fixture transport only; optional live smoke tests are ignored.

## Tests

Success, pagination, empty, malformed payload, missing fields, 404, 429/Retry-After, 5xx retry, timeout, cancellation, deterministic DTO ordering/hash, one-provider failure represented explicitly.

## Out of scope

OpenReview/DBLP, SQLite orchestration, work dedupe, candidate rank/lifecycle, surfaces.

## Verify

```bash
cargo test -p sil-api
cargo clippy -p sil-api --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Provider contract, API policy notes, fixture/test results, no commit.
