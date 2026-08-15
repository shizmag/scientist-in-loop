# Third-Party and External License Notes

`scientist-in-loop` is MIT-licensed as a first-party Rust workspace. The
license metadata is declared in the workspace `Cargo.toml`; dependencies are
separately licensed by their upstream authors. This repository does not
relicense dependency source, and dependency notices must be obtained from the
resolved Cargo packages when distributing a binary.

## Checked-in pack

`crates/sil-agent/packs/visualize-article/` is first-party MIT content. Its
manifest pins the pack revision and file digests. It generates prompts only;
any image generation is performed by an external provider after host-managed
consent and disclosure. Provider terms and licenses are outside this
repository.

## Optional ARS adapter

Academic Research Skills is an external CC-BY-NC-4.0 project. The adapter
records the upstream URL, revision, digest, attribution, and license evidence,
but does not download, vendor, or silently install ARS. A user-supplied source
and explicit acknowledgement are required. Host reports may be full, partial,
or unsupported; the adapter does not claim full ARS equivalence.

## Templates

Official venue template files are not bundled by this implementation. A user
may install a local package only when its manifest, source, license evidence,
redistribution policy, and digests permit the requested operation.
