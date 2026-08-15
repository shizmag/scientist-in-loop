# PR-B1 - Canonical venue catalogue and resolver

## Role

Venue curator/engineer. Own canonical venue identity, catalogue data, normalization, and validation.

## Goal

Replace substring venue guessing with a versioned catalogue that maps many names to stable series/edition/track identities while preserving ambiguity and raw values.

## Requirements

1. Implement pure domain types in `sil-core` (or the smallest dependency-compatible home): venue ID, kind, parent, edition, alias, external ID, collection, provenance, resolution evidence/status.
2. Add a versioned Unicode-aware normalizer: NFKC/case folding, LaTeX/HTML decoding where practical, punctuation/dash/quote normalization, `&`/`and`, whitespace collapse. It must be idempotent.
3. Exact normalized alias lookup comes first. Short aliases require declared catalogue entries/context. Return resolved/ambiguous/unknown; never tie-break silently.
4. Add catalogue validator: IDs, parents/cycles, alias collisions, validity windows, external IDs, provenance, schema/catalogue/normalizer versions.
5. Create evidence-backed catalogue source files and contribution guidance. Every alias records evidence URL/type, curator, and review date. Aliases of four characters or fewer and declared collisions require context constraints or a second independent source. Initial merge may be staged, but PR-V cannot pass until both these quality rules and the 200-300 series / 1,000+ alias target are met.
6. Include hard cases: NIPS/NeurIPS; ACL/NAACL/Findings/workshops; Nature/Nature Machine Intelligence; OpenReview platform versus venue; arXiv/CoRR versus publication venue.
7. Do not encode an undisclosed prestige tier. Explicit venue collections may be added with source and review date.

## Tests

Golden normalization/resolution fixtures, ambiguity, validity years, collisions, parent cycles, input-order stability, no substring false positives, catalogue snapshot validation.

## Out of scope

HTTP providers, DB migration, candidate ranking/UI, automatic rewriting of existing raw venues.

## Verify

```bash
cargo test -p sil-core
cargo clippy -p sil-core --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Catalogue statistics, unresolved design cases, validation/test output, no commit.
