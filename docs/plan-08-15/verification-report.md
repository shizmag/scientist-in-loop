# Stage 15 Verification Report

## Verdict

**SHIP.** The six reviewed segments merge into 249 unique venues and 1,278 deduplicated aliases, satisfying the 200-300 venue / 1,000+ alias acceptance gate. The catalogue validates, including parent references/cycles, provenance, context-constrained collisions, validity windows, and external IDs.

The acceptance test runs as part of the ordinary suite:

```text
cargo test -p sil-core venue::tests::catalogue_acceptance_target_is_met
249 venues / 1,278 aliases
```

## Added Coverage

| Contract | Offline coverage | Result |
|---|---|---|
| Draft check policy | Shared manuscript fixture; nested input, bibliography, graphic path; changed plot bytes change the fingerprint but do not add a baseline finding or block draft check | PASS |
| Venue identity | NIPS/NeurIPS validity windows, proceedings title, Nature versus Nature Machine Intelligence, exact ambiguity behavior | PASS |
| Venue target | Explicit 200-300 / 1,000+ acceptance gate | PASS: 249 / 1,278 |
| Provider failures | Existing fixture providers cover pagination, empty result, malformed payload, 404, 429/Retry-After, 5xx, timeout, cancellation, and partial result status | PASS |
| Package security | Existing traversal, duplicate, hash mismatch, symlink, archive limit, rollback, cache quota, and atomic-lock fixtures | PASS |
| Release archive | Compiled PDF requirement, member SHA-256/size manifest, source-only label, missing dependency, two-run byte identity | PASS |
| MCP conformance | Six names, typed action schemas, negotiated duplex transport, structured results, resources/prompts, explicit-root and confinement tests | PASS |
| Installer | Malformed JSON preservation, unknown-field preservation, backup, idempotency, ownership-safe uninstall, OpenCode schema, platform paths | PASS |
| Licensing boundary | Visualize Article MIT notice, external-provider disclosure, and no ARS text in the embedded pack | PASS |

## Fixture

`tests/fixtures/pr-v/` is the shared offline scenario fixture. It contains the manuscript, nested TeX input, bibliography, plot bytes, class file, config, and template/skill lock examples. The manuscript is consumed by the shared check test, CLI-facing check use case, TUI render test, and MCP resource test.

## Commands

Focused tests:

```text
cargo test -p sil-core venue::tests
cargo test -p sil-latex release::tests
cargo test -p sil-app check::tests
cargo test -p sil-mcp sdk::tests
cargo test -p sil-mcp --lib server::tests
cargo test -p sil-agent registry::tests
cargo test -p sil-api discovery::tests
cargo test -p sil-package
cargo test -p sil --bin sil mcp_install::tests
```

All focused commands passed, with the catalogue target test ignored unless explicitly selected with `--ignored`.

Required gates:

```text
cargo test --workspace                 PASS (177 sil-core tests, including 1 ignored acceptance gate)
cargo clippy --workspace --all-targets -- -D warnings  PASS
cargo fmt --all -- --check             PASS
```

## Residual Gaps

- Twenty normalized aliases remain intentionally ambiguous across distinct venue IDs; each is context-constrained and resolution remains non-selecting without matching context.
- There is no complete CLI/TUI/MCP parity assertion comparing one fixture's report fingerprint and counts across all three surfaces; the added TUI test is a render-consumption check.
- TUI repeated-render caching and estimate/status/doctor parity remain covered only indirectly or by existing unit tests.
- A real fake-compiler scenario for stale-PDF rejection and compile-failure rollback was not added in this verifier change.
- Installer capability/unsupported-host and optional-hook scenarios are not fully represented by the new fixture set.
- ARS acknowledgement and full/partial/unsupported capability reporting lack a dedicated end-to-end fixture.
- No live provider or external experiment runner was used, per scope.
