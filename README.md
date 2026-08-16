# scientist-in-loop (`sil`)

**scientist-in-loop** is a Rust CLI that turns a folder containing a scientific paper into a self-documenting, agent-friendly workspace.

Humans and LLM agents work under the same strict conventions:

- **Git** is the single source of history  
- **SQLite + FTS5** is rebuildable local memory for parsed sources and discovery snapshots
- **`structure.yaml`** is the formal syntactic map of the paper  
- **`paper_draft.tex`** is the working document (later promoted to `paper.tex`)  
- **Settings & Co-Author Cache** (Global in `~/.config/sil/`, Local in `.sil/config.yaml`) managed via Ratatui TUI (`sil settings`)
- **Versioned skill packs** are locked under `.sil/skills.lock`; managed and local skills are separate

After `sil init`, you can hand a goal to an agent and it will understand the layout, rules, and current state.

| | |
|---|---|
| **Binary** | `sil` |
| **Repository** | `scientist-in-loop` |
| **Language** | Rust (workspace) + thin Python helpers for PDF download/parse |

---

## Vision

Scientific writing with AI assistants often devolves into ad-hoc folders, lost provenance, and opaque agent context. `sil` enforces a small, boring, reliable layout:

1. Original literature sources (PDFs, Markdown `.md`, Plain Text `.txt`, HTML `.html`) live in `sources/`.  
2. Parsed text and explicit bibliographic metadata (`authors`, `year`, `venue`, `doi`, `arxiv_id`, `url`, `abstract`) are indexed in SQLite/FTS5 — never duplicated as loose markdown dumps.  
3. Extracted references from documents are auto-tagged with `% [status: unproved, incomplete]` and `note={unproved, incomplete}` until officially verified.  
4. Fetching official papers (`sil source fetch <doi|arxiv|url>`) automatically upgrades and replaces incomplete `references.bib` entries via smart deduplication (DOI, arXiv ID, title similarity).  
5. Incremental reference checking automatically validates DOIs, arXiv IDs, and OpenReview note links in `references.bib` via an Abstract Factory architecture, checking title metadata similarity, caching statuses in SQLite via Update Surgery, and providing `--fix` (`-f`) auto-repair.  
6. The high-level plan lives in `.sil/structure.yaml` with explicit section completion.  
7. Global author requisites and project co-authors are managed via a rich Ratatui TUI (`sil settings`), with automatic caching across articles.  
8. Every meaningful change produces a **commit proposal** with a `Sci-Action:` trailer — never an auto-commit.  
9. Agents load `SYSTEM.md` always, and `paper.md` / `agent-code.md` only when the task touches those surfaces.  

---

## External dependencies

`sil` is a Rust binary, but several features shell out to system tools. Install these before (or with) the project.

| Dependency | Required for | Notes |
|------------|--------------|--------|
| **Rust** (stable `cargo` / `rustc`) | Compile / install `sil` | Edition **2024** — use a recent stable toolchain via [rustup](https://rustup.rs) |
| **Git** | `init`, `status`, `log`, commit proposals | Must be on `PATH` as `git` |
| **CMake** | Building `sil` (`xberg` / native libs) | Required for Leptonica/Tesseract C++ compilation (`brew install cmake`, `apt install cmake`) |
| **Python 3** | `sil parse`, `sil source fetch` | Helpers under `python/`; override with `SIL_PYTHON` |
| **[uv](https://docs.astral.sh/uv/)** | Project Python env | Root `pyproject.toml` + `uv.lock`; install via [astral.sh/uv](https://docs.astral.sh/uv/getting-started/installation/) |
| **uv packages** (`pypdf`, optional **marker-pdf**) | PDF text extraction | `uv sync` (pypdf); `uv sync --extra marker` for Marker quality |
| **xberg** (Rust Crate) | Structured PDF Metadata & Citation Extraction | Extracts `title`, `authors`, and `citations` via LLM/NER schema. Models cached under `~/.cache/sil/models/xberg` |
| **C toolchain** | Building `sil` (bundled SQLite) | Xcode CLT (macOS), `build-essential` (Debian/Ubuntu), MinGW or MSVC (Windows) |
| **LaTeX engine** | `sil paper build` / `sil paper check --build` | Default config uses **tectonic**; also supports `latexmk`, `pdflatex`, `xelatex`, `lualatex` |

| Feature | Works without extra install? |
|---------|------------------------------|
| `sil init` / structure / SQLite | Needs **git** + built `sil` |
| `sil settings` / `sil tui` | Terminal TUI built into `sil` (powered by Ratatui & Crossterm) |
| `sil parse` | Uses **xberg** for structured metadata/citations and **Python 3** / **marker-pdf** for Markdown text |
| `sil source fetch` | Needs **Python 3** (stdlib networking) & CrossRef/arXiv APIs for official BibTeX resolution |
| `sil paper build` | Needs a **LaTeX engine** on `PATH` |


### Install script (macOS / Linux / Windows)

A single shell installer lives in `install/`. It detects the OS, installs missing tools via the local package manager when possible, then compiles and installs `sil`.

```bash
# From the repository root
chmod +x install/install.sh

# Core: git, CMake, Rust, uv, Python env (pypdf), C tools → cargo install sil
./install/install.sh

# Also install Marker (large, via uv) and a LaTeX engine for sil build
./install/install.sh --with-marker --with-latex

# Report what is present / missing (no changes)
./install/install.sh --check-only

# Dependencies only (skip cargo install)
./install/install.sh --skip-build
```

| Platform | How the script runs | Package managers used |
|----------|---------------------|------------------------|
| **macOS** | Terminal / zsh / bash | Homebrew; Xcode CLT; rustup |
| **Linux** | bash | apt, dnf/yum, pacman, zypper, or apk; rustup |
| **Windows** | **Git Bash**, **MSYS2**, or **Cygwin**; **WSL** follows the Linux path | choco / scoop / winget / MSYS2 pacman when available; rustup |

On Windows without a Unix shell, install [Git for Windows](https://git-scm.com/download/win) (includes Git Bash), then run `./install/install.sh` from the repo root inside Git Bash. WSL is recommended for the closest experience to Linux.

After install, ensure Cargo’s bin directory is on your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # macOS / Linux / Git Bash
```

Manual alternative (if you already have the toolchain):

```bash
# System packages (examples)
# macOS:  brew install git cmake python uv tectonic
# Debian: sudo apt install git cmake python3 curl build-essential
# Install uv: https://docs.astral.sh/uv/getting-started/installation/
# Then, from the repository root:
uv sync                      # pypdf into .venv
# uv sync --extra marker     # optional, heavy (Marker PDF parser)
# uv sync --group dev        # golden_dataset validate/score scripts
export SIL_PYTHON="$(pwd)/.venv/bin/python"
cargo install --path crates/sil
```

---

## Quick start

```bash
# Install deps + sil (or: cargo install --path crates/sil)
./install/install.sh

# Create a project
sil init my-paper
cd my-paper

# Configure global author info & article settings via Ratatui TUI
sil tui settings     # or: sil tui dashboard

# After upgrading the sil binary, refresh templates / .gitignore
# (preserves config, structure, manuscripts, and custom gitignore rules)
sil init --update

# Drop PDFs into sources/, then parse
cp ~/Downloads/attention.pdf sources/
sil source parse sources/attention.pdf
# or: sil source parse          # interactive multi-select of unparsed PDFs

# Open a parsed source in the TUI markdown reader
sil source read attention.pdf

# Search parsed literature
sil source search "self-attention"

# Inspect state / agent context
sil status
sil project context
sil project context --paper --agent --skill-paper

# Build the manuscript (requires configured LaTeX engine)
sil paper build

# Check current workspace invariants
sil paper check

# Install and stage a user-supplied, licensed template pack
sil paper template install ./template-pack --approve
sil paper template verify venues/example-2026
sil paper template stage venues/example-2026

# Sci-Action annotated history
sil git log
```

Optional environment:

| Variable | Effect |
|----------|--------|
| `NO_COLOR` / `SIL_NO_COLOR=1` | Disable colors |
| `SIL_NONINTERACTIVE=1` | No spinners/prompts; parse selects all |
| `SIL_PYTHON` | Python executable (default `python3`; prefer `.venv/bin/python` after `uv sync`) |
| `SIL_MARKER_BIN` | Path to pre-installed `marker_single` / `marker` CLI binary |
| `SIL_MARKER_MODE` | Parsing mode for Marker (default `balance`) |
| `SIL_MARKER_FLAGS` | Custom space-separated CLI flags for `marker_single` |
| `SIL_PARSE_SCRIPT` | Path to `parse_with_marker.py` fallback helper |
| `SIL_DOWNLOAD_SCRIPT` | Path to `download_pdf.py` |
| `SIL_MARKER_STUB` | Test-only: skip Marker, use this text |

Python helpers (`python/`) are managed with **uv** from the repo root (`pyproject.toml` / `uv.lock`). After `uv sync`, point `SIL_PYTHON` at `.venv/bin/python` or use a global `marker_single` CLI. Marker is preferred for parse quality; a pypdf fallback exists when Marker is not installed. Golden-dataset scripts need `uv sync --group dev` then `uv run tests/golden_dataset/scripts/…`.

---

## Command overview

| Command | Description |
|---------|-------------|
| `sil init [name]` | Create full project tree, templates, auto `.gitignore`, git repo, SQLite DB; **propose** first commit |
| `sil init --update` | Upgrade an existing project to the current sil templates (skills, managed `.gitignore`, missing scaffold) |
| `sil status [--json]` | Stage, git status, source counts, structure completion, draft dirty flag |
| `sil source parse [path]` | Parse PDF (via Marker) or Markdown/Text/HTML sources natively into SQLite + FTS5 |
| `sil source read <id>` | Open a parsed source document in the interactive TUI markdown reader |
| `sil source search <query>` | FTS5 full-text search over parsed sources |
| `sil source fetch <doi\|arxiv\|url>` | Download PDF, HTML, or Markdown into `sources/` via DOI (`10.xxxx`), arXiv ID, or URL |
| `sil source list [--json]` | List sources with format tags (`[pdf/parsed/on-disk]`, `[md/parsed/on-disk]`), metadata, and visibility |
| `sil source remove <id>` | Drop a source from the DB so it can be reparsed |
| `sil source cite <source\|query>` | Suggest BibTeX + `\cite{…}` incorporating stored authors, year, venue, and DOI (optional `--append` to `references.bib`) |
| `sil source digest [query]` | Legacy single-query Crossref digest; discovery runs preserve provider provenance |
| `sil source doctor [id]` | Heal parsed sources: re-extract reference entries and hydrate missing metadata via DOI |
| `sil paper check [--profile submission|--strict]` | Evaluate current-state manuscript invariants; no implicit baseline or scientific-result comparison |
| `sil paper build [--source-only]` | Compile or explicitly publish source dependencies; compilation requires a newly produced PDF when requested |
| `sil paper template list\|install\|verify\|stage\|remove` | Manage manifest-based local template packs without rewriting the workspace manuscript |
| `sil paper template apply` | Compatibility path for legacy template names; it does not install official venue files |
| `sil paper split` | Write agent-readable section files under `.sil/draft_sections/` (does not edit `paper_draft.tex`) |
| `sil paper promote [--force]` | Copy `paper_draft.tex` → `paper.tex` and propose `promote-to-final` |
| `sil paper structure list\|set` | Inspect or update section completion in `structure.yaml` |
| `sil paper todo [--json]` | List active `# -- X -- #` idea and TODO blocks parsed from `paper_draft.tex` |
| `sil paper estimate [--mode quick\|full\|methodology] [--json] [--write]` | L0 multi-perspective manuscript estimate (read-only; optional write under `.sil/reviews/`) |
| `sil paper recent [--json]` | List recently opened scientist-in-loop projects from global configuration |
| `sil paper assets [--json]` | List and validate figures, graphics, and `\input` dependencies in `paper_draft.tex` |
| `sil paper pack [-o bundle.zip]` | Generate reproducible manuscript ZIP package containing draft, structure, BibTeX, review reports, and `REPRO.md` |
| `sil project context [flags]` | Structured context dump for humans/agents |
| `sil project doctor [--json] [--fix] [--repair-db]` | Project layout, host dependencies, and manuscript health audit with actionable hints; `--fix` repairs bibliography entries and `--repair-db` backs up/rebuilds a corrupt SQLite index from on-disk sources |
| `sil paper template update` | Install an approved newer local template revision after verification |
| `sil project mcp [--project PATH] [--quiet]` | Start the SDK-backed stdio MCP server with an explicit project root; direct CWD fallback is reported |
| `sil mcp install|status|uninstall` | Safely manage the sil-owned entry for supported hosts with backup and atomic updates |
| `sil project skills list|install|verify|check|diff|approve-update|remove|rollback` | Manage locked skill packs and report full, partial, or unsupported host capability |
| `sil git log` | Git log filtered/annotated by `Sci-Action` trailers |
| `sil git propose [--action …]` | Print a Sci-Action commit proposal from dirty paths or an explicit action (never commits) |
| `sil tui dashboard` / `sil tui settings` | Interactive five-tab Ratatui TUI for the command center, sources, draft, references, and settings |

Commit proposals always include a trailer such as:

```text
Sci-Action: init
Sci-Action: parse-pdf
Sci-Action: fetch-source
```

`sil` **never** auto-commits.

### Stage 15 workflows

`sil paper check` is the deterministic workspace gate. Use the default draft
profile for current-state errors, `--profile submission` for explicit
submission blocking policy, `--strict` for all actionable warnings, and
`--json` for the complete structured report. `--online` is opt-in; provider
unavailability is not a manuscript truth claim.

Discovery preserves raw provider data and venue strings, records canonical
identity evidence and ambiguity, and places results in an explicit candidate
workflow. Collections are selected, versioned sets, not a universal ranking.

Template and skill installation is explicit and lock-backed. Existing
`apply -t neurips`-style names remain a compatibility route for old projects;
they are not official/current bundled venue packages. New workflows should
install a local manifest, approve it, verify it, and then stage it.

Visualize Article generates prompts for external image providers and does not
render figures itself. ARS is optional CC-BY-NC external content and is never
silently vendored; host support can be partial or unsupported. External
experiment execution and symlink management are deliberately not implemented
because the concrete provenance, safety, and lifecycle contract is not yet
approved.

---

## Interactive TUI Command Center & Settings (`sil tui` / `sil settings`)

`sil tui` (or `sil settings`) opens a 5-tab Ratatui interface:

The command palette is available with `:` or `Ctrl-K`. It searches named commands and aliases, including parse, fetch+parse, cite, undo, build, estimate, repair, and review actions. Empty states expose keyboard-reachable next actions instead of requiring memorized shortcuts.

1. **Dashboard (`1`)**: Live daily command center with four real-time panes:
   - **Health**: Stage, LaTeX engine & main file, manuscript audit (bib coverage ratio and unmatched label counts), word count, and active TODO counts.
   - **Ideas**: Real active `# -- X -- #` TODO and idea blocks parsed from `paper_draft.tex`.
   - **Digest**: Live literature publication digest feed backed by global/local settings query with non-blocking TUI background auto-refresh; press `Enter` on a row to queue an async source fetch.
   - **Shortcuts**: Keyboard command reference and factual project status counts.
2. **Sources (`2`)**: Comprehensive literature manager for registered documents in `sources/`:
   - Paginated pretty Markdown reader (`Enter`, `j`/`k`, `PageUp`/`PageDown`).
   - Append current source document to `references.bib` (`b`) with metadata hydration (`% [sil: tui-added]` marker) in reader or sources list.
   - Park note / capture claim from reader (`n`), then choose a draft section or end of draft; the block is tagged `from-source` with `from: <filename>` in `paper_draft.tex`.
   - Insert the selected source citation into a chosen draft section (`c`) after ensuring the source is in `references.bib`.
   - Add new works via link / URL / DOI / arXiv (`a`).
   - Real-time parse status indicator (`[✓ Parsed]` / `[Unparsed]`).
   - Derived source badges for parsed state, bibliography presence, and citation in the draft.
   - Source statistics (word count, extracted reference count).
   - Extracted references viewer per document (`v`) with single-item (`c`/`b`/`p`) or batch (`a`) append to `references.bib`.
   - Rename source titles (`r`) and delete sources with confirmation (`d` / `Delete`).
3. **References (`3`)**: Split-pane reference manager (`references.bib` vs Extracted References):
   - Switch active pane (`Tab`).
   - Search bib entries or extracted references (`/`).
   - Add selected or marked (`Space`) references to `references.bib` (`p`).
   - Promote TUI-added bib entries by stripping `% [sil: tui-added]` marker (`P`).
   - Recompute cosine similarity of extracted references against current `paper_draft.tex` (`X`).
   - Multi-field reference sorting: Similarity (`m`/`c`), Year (`y`), Venue (`v`), Source (`s`), Index (`i`), and Title (`t`).
   - Delete entries from `references.bib` (`Delete`).
4. **Paper Draft (`4`)**: Interactive section-by-section LaTeX manuscript viewer & editor with `$EDITOR` integration (`e` for TUI popup, `v` for external `$EDITOR`).
5. **Settings (`5`)**: Unified settings window with distinct section dividers:
   - **Global Settings**: Default author requisites, default grant, engine, template defaults, literature digest query (`digest_query`), and background refresh interval in hours (`digest_refresh_hours`, min 1) (`~/.config/sil/settings.yaml`).
   - **ONNX & Local RAG Settings**: Model paths, thread allocation, chunk sizes, and execution providers.
   - **Co-Author & Grant Caches**: Fast import/export alias for cached authors and grants (`~/.config/sil/cache.yaml`).
   - **Local Project Settings**: Article title, co-authors list, active grant requisites, literature digest query override (`digest_query`), and project notes (`.sil/config.yaml`).

### Keybindings in TUI

- `1`-`5` or `Tab` / `Shift+Tab`: Switch tabs (1. Dashboard, 2. Sources, 3. References, 4. Paper Draft, 5. Settings).
- `?` / `F1`: Open mode-aware keyboard help overlay showing actual shortcuts for the current view/modal context.
- `R`: Reload project sources and bibliography entries from disk.
- `↑`/`↓` or `j`/`k`: Navigate sources, fields, references, or sections.
- `Enter` / `e`: Edit selected setting field or section body, read source Markdown, parse selected unparsed source (Sources tab), or queue async source fetch for DOI/URL (Dashboard digest pane).
- `E` / `Shift+E`: Parse all unparsed source documents (Sources tab).
- `a`: Add source link / URL / DOI / arXiv (Sources tab) or add author / grant (Settings tab).
- `b`: Append selected source document to `references.bib` (Sources tab & Reader mode).
- `n`: Open note capture and section picker to insert a `# -- X -- #` block tagged `from-source` into `paper_draft.tex` (Reader mode).
- `c`: Insert the selected source's existing cite key into a chosen draft section (Reader mode).
- `r`: Rename selected source document title.
- `d` / `Delete`: Delete source document (Sources tab), delete setting item (Settings tab), or remove entry from `references.bib` (References tab).
- `v`: View extracted references for selected source (Sources tab), launch external `$EDITOR` (Paper Draft tab), or sort by venue (References tab).
- `p` / `P`: Add extracted reference to `references.bib` (`p`), or promote TUI-added entry (`P`).
- `m` / `X`: Sort references by draft cosine similarity (`m`), or recompute similarity scores (`X`).
- `y` / `i` / `s` / `t`: Sort references by Year (`y`), Index (`i`), Source (`s`), or Title (`t`).
- `u`: Copy selected cached author/grant into local project settings (Settings tab).
- `Space`: Toggle selection mark on reference item (References tab & Reference viewer).
- `/` / `f`: Search / filter references or bib entries.
- `s` or `Ctrl+S`: Save all global settings, local config, and cache.
- `q` or `Esc`: Quit TUI / close open modal or help overlay.

### TUI Recovery And Onboarding

- `Ctrl+Z` or the command palette restores the latest supported TUI file mutation from the capped, git-independent `.sil/undo/` journal.
- TUI errors expose a user-facing title and hint; background job history persists in `.sil/jobs.json` and interrupted jobs are marked stale on the next start.
- Disk-change conflict banners and PID-aware advisory lock banners make competing edits visible and require confirmation before protected mutations.
- `sil tui` without a project opens a first-run wizard for recent projects, opening a path, initialization, or doctor. `sil init --demo` creates a small offline synthetic fixture for trying the workflow.
- The TUI can open the latest estimate report, build the draft and jump to the first reported LaTeX error, show grounding results, and review an uncommitted proposal/diff. These surfaces never auto-commit.
- There is no GUI, daemon, hard `flock`, `sil daily` command, or auto-commit.

---

## Model Context Protocol (MCP) Server & Local RAG

`sil` provides a native **Model Context Protocol (MCP)** stdio JSON-RPC server (`sil project mcp` / `sil mcp`) allowing external AI IDEs and assistants (Claude Desktop, Cursor, etc.) to inspect literature, edit sections, estimate manuscript quality, manage bibliography, update `# -- X -- #` TODO blocks, fetch sources, and format commit proposals (**6 workflow-oriented tools**). Tools never auto-commit; they return Sci-Action proposals.

### Key MCP Features

1. **Hybrid RAG (`sil_sources` `action=search`)** — BM25 FTS5 always; **dense ONNX** embed/rerank only when built with `cargo build -p sil --features onnx` **and** models + `tokenizer.json` load successfully under `~/.cache/sil/models/` (otherwise honest hash/token **fallback**; see `sil project doctor`):
   - **Parent-Child Chunking**: Splits parsed Markdown literature by section headings (parent chunks) and paragraphs (child chunks).
   - **RRF + HyDE**: Reciprocal Rank Fusion and optional HyDE query expansion.
   - **Custom ONNX Paths**: Configure `onnx_models_dir` / `onnx_embedder_path` / `onnx_reranker_path` in settings; directory paths auto-locate `*.onnx`.
   - **Parent Context Expansion**: Child hits expand to full parent section context.
2. **Structured Async TODO Governance (`sil_context`, `sil_draft` `action=todo`)**:
   - Query, prioritize, and update `% # -- X -- #` comment blocks inside `paper_draft.tex` with status (`open`, `in_progress`, `resolved`), priority (`low`, `medium`, `high`, `critical`), section tags, and author provenance.
3. **Literature Fetching (`sil_sources` `action=fetch`)**:
   - Download papers/sources into `sources/` by DOI (`10.xxxx`), arXiv ID (`arxiv:XXXX.YYYY`), or direct URL, and optionally parse into SQLite FTS5 index.
4. **Commit Proposal Governance (`sil_propose`)**:
   - Generates structured commit proposals with `Sci-Action:` trailers for human review. **Never auto-commits**.

### Workflow-Oriented MCP Surface (6 Tools)

| Tool | Role | Dispatch / Action |
|------|------|-------------------|
| `sil_context` | Orient: project snapshot, skills, structure/TODOs | Flags + optional skill name/input |
| `sil_sources` | Literature lifecycle: search, get, fetch, parse, rank | `action`: `search` \| `get` \| `fetch` \| `parse` \| `rank` |
| `sil_cite` | Bibliography & claim grounding | `action`: `suggest` \| `ground` \| `upsert` \| `promote` |
| `sil_draft` | Manuscript mutations: section edit, TODOs, structure | `action`: `edit` \| `todo` \| `structure` |
| `sil_review` | Quality gates: manuscript estimate & build/doctor | `action`: `estimate` \| `build` |
| `sil_propose` | Commit proposal generation | `message` / Sci-Action category (never auto-commits) |

### Migration Table (19 Fine-Grained Tools → 6 Workflow Tools)

| Old tool | New tool | Action / notes |
|----------|----------|----------------|
| `sil_get_workspace_context` | `sil_context` | flags `include_*` |
| `sil_list_skills` | `sil_context` | list skills (e.g. `list_skills=true` or omit skill name) |
| `sil_invoke_skill` | `sil_context` | `skill` + optional `input` |
| `sil_list_todos` | `sil_context` | `include_todos` / todo filters |
| `sil_get_structure` (read) | `sil_context` | structure included or `include_structure` |
| `sil_search_sources` | `sil_sources` | `action=search` |
| `sil_get_source_context` | `sil_sources` | `action=get` |
| `sil_fetch_source` | `sil_sources` | `action=fetch` |
| `sil_parse_source` | `sil_sources` | `action=parse` |
| `sil_rank_draft` | `sil_sources` | `action=rank` |
| `sil_suggest_citations` | `sil_cite` | `action=suggest` |
| `sil_ground_claims` | `sil_cite` | `action=ground` |
| `sil_upsert_bib` | `sil_cite` | `action=upsert` |
| `sil_promote_bib` | `sil_cite` | `action=promote` |
| `sil_edit_section` | `sil_draft` | `action=edit` |
| `sil_update_todo` | `sil_draft` | `action=todo` |
| `sil_get_structure` (update) | `sil_draft` | `action=structure` |
| `sil_estimate_paper` | `sil_review` | `action=estimate` |
| `sil_build_and_doctor` | `sil_review` | `action=build` |
| `sil_propose_commit` | `sil_propose` | same args as today |

### Manuscript estimate (native CLI)

```bash
sil paper estimate --mode full          # human markdown
sil paper estimate --mode quick --json  # machine JSON
sil paper estimate --write              # save under .sil/reviews/ + commit proposal
```

Skill pack: `agent/skills/review.md` (inspired by [academic-research-skills](https://github.com/Imbad0202/academic-research-skills) reviewer methodology; sil-native text, CC-BY-NC attribution for upstream). L0 is offline heuristic; refine with an agent using the skill (L1).

### Standard MCP Configuration (`mcp.json`)

```json
{
  "mcpServers": {
    "scientist-in-loop": {
      "command": "sil",
      "args": ["mcp", "--quiet"]
    }
  }
}
```

### Durability & System Robustness (Stage 11)

- **Atomic Writes**: All durable files (`references.bib`, `paper_draft.tex`, `.sil/config.yaml`, `.sil/structure.yaml`, `.sil/workspace.lock`, `.sil/reviews/*`, global settings, cache) are updated via `sil_core::write_atomic` (writes to a PID/nanosecond temp file in the same directory, flushes via `sync_all()`, and atomically replaces the target via `fs::rename()`).
- **SQLite WAL & Integrity**: Enforces `PRAGMA journal_mode = WAL; busy_timeout = 5000;` across all database connections and reports status in `sil doctor`.
- **Data Loss-Free Re-parsing**: Reparsing source documents runs inside a single SQLite transaction without pre-deleting source rows, preserving existing index and FTS data on failure.
- **API Retries & HTTPS**: All CrossRef, DOI, arXiv, and OpenReview HTTP requests use exponential backoff retries (3 attempts), with arXiv upgraded to HTTPS.
- **TUI Panic Isolation & Async Estimate**: TUI background workers isolate thread panics via `catch_unwind`, and manuscript estimates run non-blocking on background threads.
- See [`docs/pr-plan-08-12/pr-plan.md`](docs/pr-plan-08-12/pr-plan.md) and [`ADR-013`](docs/adr/ADR-013-crash-safe-robustness.md).

---

## Layout created by `sil init`

```text
my-paper/
├── .sil/
│   ├── config.yaml          # project paths, latex engine, local settings (co-authors, grants)
│   ├── structure.yaml
│   ├── structure.example.yaml
│   ├── db.sqlite             # rebuildable local index
│   ├── checks/               # ignored check reports
│   ├── build/                # ignored staged build trees
│   ├── template.lock         # exact template package selections
│   ├── skills.lock           # exact skill package selections
│   ├── draft_sections/       # agent section cache from `sil paper split`
│   ├── improvement/         # suggestion_n improvement proposals (tracked)
├── paper_draft.tex          # source of truth for prose
├── paper.tex
├── references.bib
├── sources/                 # original PDFs only
├── data/
│   └── README.md
├── figures/
│   ├── plots/               # code-generated plots
│   │   └── README.md
│   └── images/              # external images
│       └── README.md
├── agent/
│   ├── skills/managed/       # installed, verified pack projections
│   ├── skills/local/         # user-authored skills, never overwritten
│   └── README.md
└── README.md
```

### Folder purposes

- **`sources/`** — original scientific PDFs. Nothing else. Parsed content lives only in SQLite.  
- **`data/`** — experimental / collected data; document each file in `data/README.md`.  
- **`figures/plots/`** — plots from code; list script + figure ref in the README.  
- **`figures/images/`** — external images; document origin and license.  
- **`agent/`** — helper scripts the agent writes; document purpose and how to run.  
- **`.sil/draft_sections/`** — deterministic per-section split of `paper_draft.tex` for agents (`sil paper split`); do not edit as source of truth.
- **`.sil/template.lock` / `.sil/skills.lock`** — portable exact package selections; SQLite and caches are derived state.
- **`.sil/improvement/`** — versioned improvement proposals as `suggestion_n` (not gitignored).  

### Default `.gitignore`

`sil init` writes a **sil-managed** `.gitignore` (block between `# >>> sil-managed` and `# <<< sil-managed`) that ignores large or rebuildable artifacts by default:

| Ignored | Still tracked |
|---------|----------------|
| `.sil/db.sqlite` (and other SQLite files under `.sil/`) | `.sil/config.yaml`, `structure.yaml`, skills, **improvement/**, **draft_sections/** |
| All PDFs (`*.pdf`) everywhere (literature in `sources/`, root PDFs) | `sources/README.md`, `sources/` directory shell |
| Image/media binaries (`*.jpg`, `*.png`, `*.webp`, `*.svg`, …) | Folder `README.md` files (`figures/**/README.md`, `data/README.md`) |
| Contents of `data/**` (experiment outputs), LaTeX aux files | Manuscripts (`paper_draft.tex`), `references.bib`, project README |
| Common result/cache trees (`results/`, `wandb/`, checkpoints, …) | Custom scripts in `agent/` |

Put local rules **below** the managed end marker. `sil init --update` refreshes only the managed block.

### Upgrading a project (`sil init --update`)

When you install a newer `sil`, run this inside an existing project:

```bash
sil init --update
# or: sil init --update path/to/project
```

| Always refreshed | Created only if missing | Never overwritten if present |
|------------------|-------------------------|------------------------------|
| sil-managed skill projections | Folder READMEs, layout dirs | `.sil/config.yaml` and local skills |
| `.sil/structure.example.yaml` | Paper stubs, `references.bib` | `.sil/structure.yaml` |
| sil-managed `.gitignore` block | Project `README.md` | `paper_draft.tex`, `paper.tex`, local skills, lock files |
| | SQLite DB / git repo (ensured) | Custom gitignore rules outside the managed block |

Proposes a commit with `Sci-Action: update` (never auto-committed).

---

## How agents should interact

1. Read the managed or local `agent/skills/` system skill first (always loaded by `sil context`).
2. Run `sil context` for a fresh snapshot (structure, config, Sci-Action history, sources).  
3. Load additional skills when relevant:  
   - **`paper.md`** — tasks touching `structure.yaml`, `paper_draft.tex`, `paper.tex`, or section completion.  
   - **`agent-code.md`** — tasks creating/modifying anything under `agent/`.  
4. Update `completion` in `structure.yaml` when changing a section’s status.  
5. Write new prose into `paper_draft.tex`; promote to `paper.tex` only when sections are at least `draft`.  
6. After significant work, use the **commit proposal** `sil` prints; keep the `Sci-Action` trailer.  
7. Use `# -- X -- #` blocks in `paper_draft.tex` to communicate ideas or TODOs with human scientists. Never auto-commit.

### Idea & TODO Blocks (`# -- X -- #`)

Both human researchers and AI agents can bound ideas, questions, or revision notes directly inside `paper_draft.tex`:

```latex
% # -- X -- #
% TODO: Re-evaluate section 3 baseline comparisons using the new dataset.
% Idea: Add an ablation table comparing model A vs model B.
% # -- X -- #
```

`sil` automatically parses these blocks into SQLite memory. They are surfaced in:
- `sil tui dashboard` (TUI Command Center)
- `sil todo` (CLI list of active ideas)
- `sil context` (automatically loaded into AI agent context)

Flags for richer context:


```bash
sil context --paper              # deterministic LaTeX section split of paper_draft.tex
sil context --agent              # agent/ listing + README
sil context --skill-paper
sil context --skill-agent-code
sil context --task "edit introduction in paper_draft.tex"
```

---

## Workspace architecture (developers)

```text
crates/
  sil/          # binary only — clap + wiring
  sil-api/      # external API interactions (Crossref, arXiv, OpenReview) & rate limiting
  sil-app/      # application use-cases (upsert_bib, promote_bib, fetch_source)
  sil-core/     # domain types, Config, settings, errors, paths, terminal UX
  sil-db/       # SQLite + FTS5 + update surgery for bib verifications
  sil-git/      # status, commit proposals, Sci-Action trailers
  sil-parse/    # PDF validation, Marker orchestration, Abstract Factory reference checkers (DOI, arXiv, OpenReview)
  sil-regex/    # centralized regular expressions and pattern matchers (DOI, arXiv, OpenReview)
  sil-latex/    # engine abstraction + section splitter
  sil-agent/    # dynamic skills + context generation
  sil-template/ # ML/AI conference article templates (NeurIPS, ICML, ICLR, IEEE, arXiv)
  sil-tui/      # Ratatui TUI for global/local settings and co-author cache
python/         # download_pdf.py, parse_with_marker.py
templates/      # files copied by sil init
docs/           # Architectural Decision Records (ADRs) & technical notes
```

- Domain logic lives in libraries; the binary stays thin.  
- `thiserror` in libraries, `anyhow` in the binary.  
- Paths use `camino`.  
- Terminal colors/progress go through a testable `SilUi` abstraction (`NullUi` in tests).

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p sil -- --help
```

---

## Current MVP status

| Area | Status |
|------|--------|
| Multi-crate workspace + domain types | Done |
| `sil init` exact layout + templates + managed `.gitignore` + git + SQLite | Done |
| `sil init --update` template upgrade for existing projects | Done |
| Typed `config.yaml` / `structure.yaml` + `sil status` | Done |
| `sil settings` / `sil tui` interactive Ratatui TUI for global/local settings & co-author cache | Done |
| `sil parse` (path + noninteractive multi-select) + FTS5 `sil search` | Done |
| Multi-format source probing (`PDF`, `Markdown`, `Text`, `HTML`) | Done |
| SQLite bibliographic metadata (`authors`, `year`, `venue`, `doi`, `abstract`) | Done |
| Incremental BibTeX reference verification (DOI, arXiv, OpenReview) + Title Mismatch & `--fix` Autofix | Done |
| Rich BibTeX generation (`sil cite`) | Done |
| Marker via Python helper (stubbable for tests) | Done |
| Commit proposals + `sil log` Sci-Action trailers | Done |
| `sil paper build` / `sil source fetch` / `sil project context` + skills | Done |
| `sil paper template` package lifecycle + legacy apply compatibility | Done |
| Stage 15 check, discovery, package, MCP, and skill trust boundaries | Verification-gated; see `docs/plan-08-15/verification-report.md` |
| Colored output + progress (disabled in tests) | Done |
| Auto-commit | **Never** (by design) |
| Engines beyond Marker | Out of scope for MVP |

---

## License

MIT
