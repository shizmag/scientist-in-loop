# ADR-018: Venue Identity and Discovery Evidence

## Status

Accepted for Stage 15; ship status is gated by `docs/plan-08-15/verification-report.md`.

## Decision

Venue records use stable canonical IDs. Raw venue text is retained alongside
the alias, catalogue version, normalizer version, and evidence. Resolution is
`resolved`, `ambiguous`, or `unknown`; short or colliding aliases are never
silently guessed. Series, edition, track, journal, repository, and hosting
platform remain distinct concepts.

Discovery stores immutable provider request and response snapshots with hashes,
cursors, retrieval metadata, and provider status. Candidate ranking is
versioned, deterministic, and explained by stored components. A venue
collection is an explicitly selected, versioned set with provenance and review
date, not a universal prestige, quality, or "top venue" claim.
