# Integration / e2e tests

End-to-end tests that drive the real `sil` binary live under the binary crate
(so `assert_cmd` can resolve `CARGO_BIN_EXE_sil`):

```text
crates/sil/tests/
├── common/mod.rs      # shared runner helpers (plain / noninteractive)
├── e2e_init.rs
├── e2e_status.rs
├── e2e_parse.rs
├── e2e_search.rs
├── e2e_context.rs
├── e2e_git.rs
└── e2e_build.rs
```

Unit tests stay next to library code (`#[cfg(test)]` modules in each crate).

All e2e runs force:
- `SIL_NO_COLOR=1` / `NO_COLOR=1`
- `SIL_NONINTERACTIVE=1`
- `SIL_MARKER_STUB=…` (no live Marker required)
