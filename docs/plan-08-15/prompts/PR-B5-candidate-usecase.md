# PR-B5 - Candidate identity, lifecycle, dedupe, and ranking use case

## Role

Candidate engineer. Own `sil-app` discovery orchestration and candidate policy.

## Goal

Fan out provider requests, resolve venues, conservatively identify publication versions, persist candidates/events, and rank a frozen run with transparent stable components.

## Requirements

1. Implement `sil-app` discovery use case over B1-B4 and B2 repositories, with bounded provider concurrency, cancellation, provider-specific errors, resumable cursors, and partial-run status.
2. Persist immutable request/record provenance before normalization/merge policy.
3. Identity order: normalized DOI, arXiv base+version, OpenReview forum/cross-ID, provider cross-IDs. Title/author/year similarity proposes relations but does not auto-merge publication versions.
4. Apply venue resolver and preserve raw/status/evidence/catalogue/normalizer versions.
5. Candidate resolution/disposition/acquisition transitions are explicit append-only events.
6. Implement versioned fixed-point ranking with stored components: lexical relevance, exact phrase, requested venue collection match, provider/identifier consensus, recency, observed citation signal, OA availability. No hidden prestige.
7. Stable sort: score desc, year desc/null last, normalized title, stable work ID.
8. Discovery/shortlist/dismiss do not mutate `references.bib`. Existing fetch/upsert use cases are called only by explicit acquisition/bib actions.

## Tests

Provider partial failure, resume, input permutation, ties, missing year/citations, venue ambiguity, DOI/arXiv/OpenReview/version relations, false-merge guards, lifecycle invalid transitions, ranking component goldens, no bib mutation.

## Out of scope

CLI/TUI/MCP rendering, new provider HTTP, prestige tiers, LLM ranking.

## Verify

```bash
cargo test -p sil-app -p sil-db
cargo clippy -p sil-app -p sil-db --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Identity/ranking policy, golden outcomes, risks, no commit.
