# Design: Bib Lifecycle, TUI UX, Parse/Digest Hardening

**Project:** scientist-in-loop (`sil`)  
**Mode:** Design doc + PR DAG only (implementation deferred)  
**Date:** 2026-08-04  
**Status:** Draft for review → agent-executor handoff later

---

## 1. Problem statement

The workspace already has a solid scientific-writing loop (init → parse → sources DB → TUI references → BibTeX → build/release). Recent work added official BibTeX resolution, draft–ref similarity, TUI-added markers, and background hydration (ADR-009). Three product gaps remain tightly coupled:

1. **BibTeX lifecycle is inconsistent** — formatting, upsert policy, cite-key stability, CLI vs TUI markers, and promote/hydrate races are not fully specified or safe.
2. **TUI UX is power-user only** — Sources workflows are incomplete or dishonest (`a` does not truly fetch), background jobs only write ephemeral status, and keyboard help is missing/stale.
3. **Parse/digest quality still has sharp edges** — one golden negative-pattern FAIL remains; official resolution hard-fails on bad DOIs; journal digest dual-stacks (native vs Python) drift.

This design unifies those tracks into one ordered PR plan so an executor agent can implement without re-discovering architecture.

---

## 2. Goals & non-goals

### Goals

| Track | Goal |
|-------|------|
| **A. Bib lifecycle** | Pretty multiline BibTeX everywhere; completeness-aware upsert; stable cite keys on hydrate; safe promote/hydrate; consistent release strip semantics |
| **B. TUI UX** | Honest Sources ingest; visible background-job feedback; first-class `?` keymap help; Sources parse/reload where practical |
| **C. Parse/digest** | 0 golden negative-pattern pollution; resilient official resolve fallback; journal digest native/Python parity |

### Non-goals (this phase)

- MCP write path for `references.bib`
- Full keybinding redesign / prefix-map overhaul (document collisions; optional later PR)
- Non-blocking similarity recompute (follow-on after job-status chrome)
- Replacing xberg or Marker; only clarify when MD block wins
- Large parent-author F1 campaign beyond fixtures already nearly passing
- Graphite/stack automation specifics (plain PR DAG is enough)

---

## 3. Current state (code truth)

### 3.1 Bib lifecycle (sil-core + writers)

**Core:** `crates/sil-core/src/bib.rs`, stubs/gates in `source.rs`.

**WIP already in tree (uncommitted):**
- `pretty_format_bibtex` → wired into `upsert_bib_entry`, `mark_tui_added_bib_entry`, `fetch_bibtex_by_doi`
- Unit test for single-line Crossref-style entry
- `should_attempt_metadata_fetch` tests consolidated in `source.rs`

**Writers of `references.bib`:**

| Path | Marks `% [sil: tui-added]`? |
|------|----------------------------|
| TUI Sources `b`, Refs `p`, viewing-refs `c`/`b`/`p`/`a` | Yes |
| TUI hydration poll success | Re-marks |
| TUI promote `P` | Removes |
| `sil cite --append` / `sil source fetch` | **No** |
| `sil build release` / submission zip | Strips tui-added only |

**Critical gaps:**
1. `upsert_bib_entry` **always replaces** on `is_same_paper` — ignores completeness (docs claim prefer complete).
2. Hydration can **change cite keys** (stub slug → publisher key) → draft `\cite{}` breakage.
3. **Promote-then-hydrate race:** background success re-applies tui-added after user promote.
4. Concurrent RMW writes without lock (last writer wins).
5. arXiv path not pretty-formatted until next upsert; arXiv version (`vN`) normalization incomplete.
6. Source hydration dedup omits pure arXiv keys.

### 3.2 TUI (sil-tui)

**Tabs (code):** Dashboard | Sources | References | Paper Draft | Settings  
**Monolith:** `app.rs` + `ui.rs`; no help module.

**Sources:** list, read MD, rename, delete (DB+file), `b`→bib+hydrate, `a` “fetch” is **stub register only**, **no parse key**, no reload after external CLI parse.

**Background jobs:** ADR-009 hydration works non-blocking; status is last-message-wins; no queue badge; no failure history.

**Discoverability:** footer titled “Status & Help” shows status only; no `?` overlay; Dashboard shortcuts and README **stale** (4 tabs, wrong keys).

### 3.3 Parse / official bib / digest

**Golden candidate scorecard:** nearly all gates PASS; **only FAIL** = Ref negative patterns (1/1035) — BEE-RAG line-wrap continuation (`must_not_extract_as_reference`).

**Official resolve** (`journal_digest.rs`): DOI → arXiv → Crossref title. If DOI is set and fails, **no fallback**. No Crossref score threshold. DOI pretty-formats; arXiv does not.

**Journal digest:** CLI always passes Python script path → native Crossref path never used from CLI; native lacks `type:journal-article` filter parity.

---

## 4. Target architecture

### 4.1 Bib lifecycle policy (normative)

```text
Extracted ref / source
  → local stub (to_bibtex / suggest_from_*)
  → [TUI] mark tui-added + pretty + upsert → disk immediately
  → if resolvable IDs: background resolve
        success: pretty official → preserve cite_key if existing entry
                 re-mark tui-added ONLY if block still tui-added
        failure: keep stub; record failure in job log
  → promote: strip tui-added (user declares permanent)
  → release: strip remaining tui-added from package only
```

**Upsert rules (replace current “always replace”):**

1. Match via `is_same_paper` (enhance arXiv: strip `v\d+` after id normalize).
2. Prefer **complete** over **incomplete** (use `BibEntryInfo.is_incomplete`).
3. Never replace complete non-tui entry with incomplete stub unless explicit force flag (CLI only, default off).
4. When replacing, **preserve existing cite_key** by default (`preserve_cite_key: true` on hydrate path).
5. Always run `pretty_format_bibtex` on any written entry block.
6. Serial write queue or file lock for all `references.bib` writers in process (TUI: channel of write ops).

**Marker policy:**
- TUI-originated adds keep `% [sil: tui-added]` until promote.
- CLI append/fetch: document as permanent by default (no marker); optional `--draft` flag adds marker for symmetry (optional PR).
- Hydration must not re-mark after promote (re-read block by paper identity; if unmarked, leave unmarked).

### 4.2 TUI job feedback & help

**Job status model (shared pattern for hydration, later parse/fetch):**

```rust
// conceptual
struct BackgroundJobs {
  in_flight: HashSet<String>,
  recent: VecDeque<JobOutcome>, // capped, e.g. 20
  last_summary: String,         // "hydrating: 2 | last: 3 ok, 1 fail"
}
```

- Footer: show `hydrating: N` while non-empty; on batch complete show aggregate.
- Optional `J` modal: recent outcomes + retry failed (phase 2 if time).
- Left bib badges: tui-added vs permanent (visual only).

**Help:**
- `?` opens mode-aware keymap overlay (Sources / Refs left / Refs right / ViewingRefs / Reading / Settings).
- Footer second line: top 4–6 context keys (not fake “Help” without content).
- Fix Dashboard “Daily Scientist Helper Shortcuts” + README tab/key drift.

**Sources honesty:**
- Short term: retitle `a` modal to “Register link / DOI / arXiv (metadata stub)” **or** wire real download via existing `fetch` + background job.
- Preferred design: `a` triggers real fetch pipeline (DOI/arXiv/URL) with same job chrome; on success `reload_sources`.
- Add `R` reload sources from disk+DB.
- Add parse action (`P` on Sources conflicts with promote on Refs — prefer `Ctrl+P` or `e` “extract/parse” on Sources only).

### 4.3 Parse / resolve / digest

**Segmentation (BEE-RAG FAIL):**
- Continuation detector: if candidate “new entry” line looks like venue/continuation (`In *…*`, `pp.`, `vol.`, mid-sentence title fragment) and previous entry incomplete, join rather than split.
- Regression: golden BEE-RAG + unit test on the exact wrap substring.

**Official resolve:**
- On DOI fail/error → continue to arXiv → title (do not hard-stop).
- Crossref: require title similarity ≥ threshold (e.g. token Jaccard or normalized containment) before accept; else Failed with reason.
- Pretty-format arXiv and all resolve outputs at source.
- Optional session cache DOI→bib (memory); 429 exponential backoff (small).

**Journal digest:**
- CLI: try native first with Python fallback (or feature flag).
- Native: add `filter=type:journal-article` (and sort parity with Python).
- Document single source of truth in pipeline doc.

---

## 5. PR plan (DAG)

Dependencies flow left → right. Parallel PRs noted.

```text
PR-A1 Format foundation
   │
   ├─► PR-A2 Upsert completeness + arXiv normalize
   │      │
   │      └─► PR-A3 Cite-key stability + hydrate marker rules
   │             │
   │             └─► PR-A4 TUI write serialization + promote/hydrate race fix
   │
PR-B1 Help overlay + doc truth ──────────────┐  (parallel with A*)
PR-B2 Job status chrome ─────────────────────┤  (after A4 preferred, can start after A1)
PR-B3 Sources honest fetch + reload ─────────┘  (uses B2 job model)
PR-B4 Sources parse action (optional)           (after B2/B3)

PR-C1 Golden negative-pattern close             (parallel early)
PR-C2 Resolve fallback + confidence             (after A1 pretty; parallel with A2)
PR-C3 Journal digest native parity              (parallel)
PR-C4 Docs/ADR consolidation                    (last)
```

### PR details

| ID | Title | Scope | Acceptance |
|----|-------|-------|------------|
| **PR-A1** | Pretty BibTeX foundation | Finish WIP: `pretty_format_bibtex`, wire upsert/mark/DOI/arXiv fetch, unit tests | Multiline entries on upsert; `cargo test -p sil-core`; no single-line DOI leftovers after upsert |
| **PR-A2** | Completeness-aware upsert | Honor `is_incomplete`; never demote complete→incomplete; arXiv `vN` normalize in `is_same_paper` | Unit tests for replace/keep matrices; docstring matches code |
| **PR-A3** | Cite-key stability | Hydrate preserves existing cite_key; policy documented | Hydrate does not change key of existing stub; e2e or unit |
| **PR-A4** | Hydration races | Serial bib writes in TUI; re-mark only if still tui-added; arXiv source dedup keys; failed write status | Promote during in-flight hydrate stays permanent; tests |
| **PR-B1** | `?` help + truth | Overlay per mode; footer keys; Dashboard + README fix | `?` works on Sources/Refs; no false `[t]itle` unbound; README tabs match |
| **PR-B2** | Background job chrome | `hydrating: N`, aggregate outcomes, recent log buffer | Batch add shows count; completion summary not last-only |
| **PR-B3** | Sources ingest | Real fetch **or** honest labeling + reload `R` | Documented behavior matches status strings; reload works |
| **PR-B4** | Sources parse (stretch) | Queue parse selected source; refresh badge | Optional; only if B2/B3 stable |
| **PR-C1** | BEE-RAG continuation | Join wrap lines; golden 0 pollution | candidate scorecard negative patterns PASS |
| **PR-C2** | Resolve reliability | Fallback chain; title confidence; pretty all paths | Bad DOI still attempts title; unit/mocks where possible |
| **PR-C3** | Digest parity | Native filter + CLI fallback order | `sil digest` works without Python when network OK |
| **PR-C4** | Docs pass | ADR-009 update, similarity doc, pipeline doc, README markers | Docs match code |

**Suggested first merge wave (minimal valuable vertical):** A1 → C1 → B1 → A2 → A3 → A4 → B2 → C2 → B3 → C3 → C4.

---

## 6. Testing strategy

| Layer | What |
|-------|------|
| Unit | `pretty_format`, upsert matrices, `is_same_paper` arXiv versions, continuation join, resolve fallback with stubbed HTTP if present |
| Golden | Re-run `golden_dataset_eval` + score script; C1 must clear negative-pattern gate |
| TUI | Existing hydration unit tests in `app.rs`; extend for promote race + job summary |
| E2E | Existing `e2e_cite`, `e2e_source`, `e2e_build` release strip; add cite-key stability case if CLI path exists |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |

---

## 7. Risks & open decisions

| Risk / decision | Recommendation (default) | Needs user override? |
|-----------------|--------------------------|----------------------|
| CLI `cite --append` should mark tui-added? | No (permanent); optional `--draft` later | Soft |
| Sources `a` = real fetch vs honest stub | Prefer real fetch reusing `sil-parse` fetch | **Yes if costly** |
| Cite-key always preserve vs rewrite draft cites | Preserve key on hydrate | Soft |
| Crossref similarity threshold | Start ~0.6 normalized token Jaccard | Soft |
| Bind left-pane bib delete | Yes with confirm (`d` when left focused) | Soft |
| Similarity `X` non-blocking | Defer to follow-on | Soft |

---

## 8. Deliverables of design phase

1. This plan (design + PR DAG).
2. **Per-PR autonomous agent prompts** under [`prompts/`](./prompts/) — one file per PR, copy-paste ready.
3. Dispatch index: [`prompts/README.md`](./prompts/README.md) (waves, deps, product defaults).
4. Optional later ADR: `docs/adr/ADR-010-bib-lifecycle-tui-jobs-and-parse-hardening.md`.

---

## 9. Autonomous agent prompts

**Location:** [`docs/pr-plan-08-04/prompts/`](./prompts/)

| PR | Prompt file |
|----|-------------|
| PR-A1 Pretty BibTeX | [prompts/PR-A1-pretty-bibtex.md](./prompts/PR-A1-pretty-bibtex.md) |
| PR-A2 Upsert completeness | [prompts/PR-A2-upsert-completeness.md](./prompts/PR-A2-upsert-completeness.md) |
| PR-A3 Cite-key stability | [prompts/PR-A3-cite-key-stability.md](./prompts/PR-A3-cite-key-stability.md) |
| PR-A4 Hydration races | [prompts/PR-A4-hydration-races.md](./prompts/PR-A4-hydration-races.md) |
| PR-B1 Keyboard help | [prompts/PR-B1-keyboard-help.md](./prompts/PR-B1-keyboard-help.md) |
| PR-B2 Job status chrome | [prompts/PR-B2-job-status-chrome.md](./prompts/PR-B2-job-status-chrome.md) |
| PR-B3 Sources ingest | [prompts/PR-B3-sources-ingest.md](./prompts/PR-B3-sources-ingest.md) |
| PR-B4 Sources parse (stretch) | [prompts/PR-B4-sources-parse.md](./prompts/PR-B4-sources-parse.md) |
| PR-C1 Golden negatives | [prompts/PR-C1-golden-negatives.md](./prompts/PR-C1-golden-negatives.md) |
| PR-C2 Resolve reliability | [prompts/PR-C2-resolve-reliability.md](./prompts/PR-C2-resolve-reliability.md) |
| PR-C3 Digest parity | [prompts/PR-C3-digest-parity.md](./prompts/PR-C3-digest-parity.md) |
| PR-C4 Docs | [prompts/PR-C4-docs.md](./prompts/PR-C4-docs.md) |

**Wave 0 (parallel):** A1 · C1 · B1 · C3 — see [prompts/README.md](./prompts/README.md).

---

## 10. Recommended next user step

1. Start **Wave 0** with four agents using the prompt files above (isolated worktrees).
2. Merge Wave 0, then continue waves per `prompts/README.md`.
3. Say **“dispatch wave 0”** to have an orchestrator spawn those agents.
