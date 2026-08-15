# PR-C2 - Template package install, lock, and staging

## Role

Template engineer. Build template-specific semantics on C1.

## Goal

Replace enum-only hard-coded rendering with validated `template.yaml` packages, explicit install/verify/update/remove, exact `.sil/template.lock`, and isolated staging that leaves workspace files unchanged.

## Requirements

1. Implement the template manifest contract from plan Section 6.4: metadata/source/license/files/entrypoint/adapter/build/constraints and normative `redistribution.bundled_with_sil` / `local_cache` / `release_archive` / evidence fields. Values are `allowed`, `user_supplied_only`, or `forbidden`; unknown permission fails closed for bundling/archive behavior.
2. Add template registry/use cases in `sil-app` and thin CLI commands for list/show/install/verify/update/remove/stage.
3. Fetch/verify/license-display/approve/atomic-lock is the install order. Tests use local fixture packs/transport.
4. Materialize a staging tree from immutable cache. Insert manuscript content through a declared safe adapter/anchor; reject missing/duplicate anchors.
5. Staging must not rewrite `paper_draft.tex`, `references.bib`, or tracked workspace assets.
6. Provide one legal standard fixture pack and a documented compatibility path for legacy names. Do not bundle unverified official venue files.
7. Expose constraints to later check/release PRs without enforcing submission policy here.

## Tests

Schema failures, compatibility/hash/license, lock stability, stage exact inventory, anchor errors, workspace byte identity, dirty cache/projection refusal, legacy alias behavior, unsupported engine.

## Out of scope

Dependency-complete ZIP, compile hard gate, skill packs, downloading copyrighted template files in CI.

## Verify

```bash
cargo test -p sil-template -p sil-app -p sil
cargo clippy -p sil-template -p sil-app -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Commands/manifest behavior, fixture pack, migration notes, tests, no commit.
