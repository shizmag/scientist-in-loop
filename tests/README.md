# Integration / e2e tests

End-to-end tests that drive the real `sil` binary live in:

```text
crates/sil/tests/e2e_init.rs
```

They use `assert_cmd` against temporary directories with
`SIL_NO_COLOR=1`, `SIL_NONINTERACTIVE=1`, and `SIL_MARKER_STUB` so output is
deterministic (no colors, no spinners, no live Marker dependency).

Unit tests live next to the library code in each crate (`#[cfg(test)]` modules).
