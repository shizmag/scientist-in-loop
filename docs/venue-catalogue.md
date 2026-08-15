# Venue catalogue

`sil-core::venue` owns canonical venue identity. The raw provider or bibliography
string remains the caller's data; resolution adds a stable ID and evidence.

The checked-in catalogue is `crates/sil-core/data/venues.yaml` (`2026.08.15`).
It contains 249 venues and 1,278 deduplicated aliases from the six reviewed
segments. The acceptance gate requires 200-300 venues and at least 1,000
aliases. Every alias needs a source URL,
evidence type, curator, and review date. An alias with four or fewer normalized
characters, or an alias shared by multiple IDs, also needs context constraints
or a second independent provenance record.

Use `Catalogue::resolve` for exact normalized matching. Resolution never uses
substring matching and never selects a candidate from an ambiguity. Venue
series, editions, tracks, journals, repositories, and hosting platforms have
separate kinds; OpenReview and arXiv therefore do not prove a publication venue.

Contributions should add an alias to the smallest canonical identity, record
validity years when historical usage changes, and run:

```text
cargo test -p sil-core
cargo clippy -p sil-core --all-targets -- -D warnings
```
