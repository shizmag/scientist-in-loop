# PR-B4 - OpenReview and DBLP conference discovery

## Role

Conference provider engineer. Own conference-specific provider evidence.

## Goal

Add OpenReview and DBLP discovery adapters that distinguish hosting, submission, acceptance, venue series, editions, workshops, and proceedings evidence.

## Requirements

1. B3 is a hard dependency. Implement against its landed provider/transport contracts; do not fork HTTP/retry abstractions.
2. OpenReview supports search/pagination and records forum/note/invitation/group/domain/content evidence. Hosting alone never means accepted.
3. Acceptance state must cite the invitation/content/group evidence and may be unknown/ambiguous.
4. DBLP maps stream/proceedings records to B1 external IDs and preserves raw venue/proceedings text and edition/year.
5. Keep workshops/tracks distinct from parent conference series.
6. Partial/malformed provider data remains inspectable and does not disappear into empty success.
7. Required tests are fixture-only.

## Tests

Accepted/rejected/withdrawn/unknown OpenReview fixtures; v2/v1 compatibility where supported; invitation ambiguity; DBLP stream/year/workshop; pagination, malformed XML/JSON, rate errors, stable raw hashes, venue alias evidence.

## Out of scope

Candidate merge/rank, bibliography writes, UI, scraping HTML pages, claiming OpenReview acceptance without evidence.

## Verify

```bash
cargo test -p sil-api
cargo clippy -p sil-api --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Evidence semantics, fixtures, unsupported states, tests, no commit.
