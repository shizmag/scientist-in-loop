# ADR-005: Pre-installed Marker CLI Integration and Parse UX

## Context
Previously, `sil parse` relied strictly on locating a Python script `python/parse_with_marker.py` via `python3` or `SIL_PARSE_SCRIPT`. In environments where `marker-pdf` was installed globally or in virtualenvs (providing `marker_single` in PATH), `sil parse` failed to discover the runner and required manually specifying Python script paths. Additionally, status output during parsing provided minimal feedback.

## Decision
1. **Direct `marker_single` CLI Discovery (`CliMarkerRunner`)**:
   - Introduced `CliMarkerRunner` in `sil-parse` which discovers pre-installed `marker_single` or `marker` binaries in system PATH (or via `SIL_MARKER_BIN`).
   - `CliMarkerRunner` spawns `marker_single <pdf> --output_dir <temp_dir> --output_format markdown --disable_image_extraction` using isolated temporary directories (`tempfile::tempdir()`).
   - Reads the generated markdown content from the temporary folder and cleans up temp files automatically on `Drop`.

2. **Unified Runner Resolution**:
   - Priority sequence: `SIL_MARKER_STUB` (tests) -> `SIL_MARKER_BIN` / PATH (`marker_single`/`marker`) -> `SIL_PARSE_SCRIPT` / `python/parse_with_marker.py` -> helpful installation error message.

3. **Pretty Human-Readable Status UX**:
   - Upgraded `sil parse` terminal UX to show live spinners during text extraction, Crossref DOI hydration, and reference parsing.
   - Outputs a visual status card summarizing Title, Authors, DOI, Character/Word counts, Reference entry counts, Duration, and SQLite/FTS5 status.

4. **Mode Balance Configuration (`SIL_MARKER_MODE`)**:
   - Added support for `mode: balance` in `ParsingConfig` (`sil.yaml` under `parsing.mode: balance`) and environment overrides (`SIL_MARKER_MODE` defaulting to `"balance"` and `SIL_MARKER_FLAGS`).

## Consequences
- `sil parse` works seamlessly with standard `pip install marker-pdf` CLI installations out-of-the-box.
- Configurable parsing modes (defaulting to `balance`).
- No temporary files linger on disk after parsing.
- Maintains full backwards compatibility for test stubs (`SIL_MARKER_STUB`) and custom Python helper scripts (`SIL_PARSE_SCRIPT`).
