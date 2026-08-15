# PR-C3 - Dependency-complete staged submission release

## Role

Release engineer. Own staged compile, archive closure, and release provenance.

## Goal

Build a submission entirely in an isolated template staging tree, require a real successful compile for normal release, and publish a deterministic dependency-complete archive with `SIL-RELEASE.json`.

## Requirements

1. Use C2 staging and A3 structured build; never temporarily rewrite workspace manuscript/bibliography.
2. Normal release fails on compile failure, missing new PDF, missing dependency, hash mismatch, or path escape.
3. `--source-only` is explicit and its manifest must say compilation was not performed/succeeded.
4. Archive closure follows A2/A3 reachable TeX/assets/bib/style/class dependencies in the staging tree. Include only declared/reachable extras; report omissions.
5. Emit `SIL-RELEASE.json`: schema, project/source revision if available, input fingerprint, template/package lock digests, engine/version, compile status, every member/hash/size/mode, exclusions.
6. Normalize member order, timestamps (`SOURCE_DATE_EPOCH`), path separators, and permissions for byte reproducibility.
7. Publish output atomically only after validation. Failed runs leave no successful-looking final ZIP.
8. Reconcile existing `paper pack`/release behavior without silently changing command promises; document migration for PR-Z.

## Tests

Nested dependency closure, missing dependency, compile fail, zero/no PDF, source-only label, workspace byte identity, symlink escape, archive member hashes, two identical runs byte-equal, changed input changes manifest/hash, atomic failed publish.

## Out of scope

Experimental reproducibility, container capture, uploading/submitting, third-party template redistribution.

## Verify

```bash
cargo test -p sil-latex -p sil-template -p sil-app -p sil
cargo clippy -p sil-latex -p sil-template -p sil-app -p sil --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Release contract, reproducibility proof, failure semantics, no commit.
