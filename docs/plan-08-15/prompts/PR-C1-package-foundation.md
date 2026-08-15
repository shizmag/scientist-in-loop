# PR-C1 - Package manifest, lock, cache, and confinement foundation

## Role

Package-security engineer. Own shared package transport primitives, not template/skill semantics.

## Goal

Create the common trusted foundation for template and skill packs: manifest envelope, exact lock records, content-addressed cache, hashes, compatibility, license metadata, atomic resolution, and path confinement.

## Requirements

1. Add a new leaf crate `crates/sil-package` for the shared package envelope, lock, hashing, cache, and confinement primitives. Keep template and skill component schemas distinct for later PRs; add only the required workspace registrations outside the crate.
2. Require package ID/version/kind, source/revision, license, compatibility, declared files, SHA-256, and capabilities.
3. Implement deterministic lock serialization and XDG content-addressed storage.
4. Reject absolute paths, `..`, duplicate normalized paths, symlink escapes, unsupported schema versions, hash mismatch, and files missing from the manifest.
5. Cache content is immutable/read-only by contract. Lock replacement and cache metadata writes are atomic.
6. Support local-directory/archive fixtures first. Remote fetching may be an injected transport but must not be required by tests.
7. Enforce plan KD-C10 before/during archive extraction: compressed/extracted/per-file bytes, file count, path depth, compression ratio, bounded time, and explicit cache quota. Manifests cannot raise limits; locked packages are not auto-evicted.
8. Do not execute package content, install dependencies, or invent a generic plugin runtime.

## Tests

Manifest/lock round-trip, input-order-stable lock, hash mismatch, traversal and symlink escape, archive bomb/size/file-count/depth/timeout limits, cache quota and locked retention, interrupted/failed install leaves old lock, compatibility rejection, license/provenance required, offline cache hit.

## Out of scope

`template.yaml`, skill routing, MCP installer, third-party pack content, signatures beyond an explicit status model.

## Verify

```bash
cargo test -p sil-package
cargo clippy -p sil-package --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

API/storage layout, security tests, unsupported cases, no commit.
