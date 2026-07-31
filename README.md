# scientist-in-loop (`sil`)

**scientist-in-loop** is a Rust CLI that turns a folder containing a scientific paper into a self-documenting, agent-friendly workspace.

Humans and LLM agents work under the same strict conventions:

- **Git** is the single source of history  
- **SQLite + FTS5** is the memory for parsed sources  
- **`structure.yaml`** is the formal syntactic map of the paper  
- **`paper_draft.tex`** is the working document (later promoted to `paper.tex`)  
- **Settings & Co-Author Cache** (Global in `~/.config/sil/`, Local in `.sil/config.yaml`) managed via Ratatui TUI (`sil settings`)
- **Skills** in `.sil/skills/` are loaded dynamically according to clear rules  

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
2. Parsed text and explicit bibliographic metadata (`authors`, `year`, `venue`, `doi`, `abstract`) are indexed in SQLite/FTS5 — never duplicated as loose markdown dumps.  
3. The high-level plan lives in `.sil/structure.yaml` with explicit section completion.  
4. Global author requisites and project co-authors are managed via a rich Ratatui TUI (`sil settings`), with automatic caching across articles.  
5. Every meaningful change produces a **commit proposal** with a `Sci-Action:` trailer — never an auto-commit.  
6. Agents load `SYSTEM.md` always, and `paper.md` / `agent-code.md` only when the task touches those surfaces.

---

## External dependencies

`sil` is a Rust binary, but several features shell out to system tools. Install these before (or with) the project.

| Dependency | Required for | Notes |
|------------|--------------|--------|
| **Rust** (stable `cargo` / `rustc`) | Compile / install `sil` | Edition **2024** — use a recent stable toolchain via [rustup](https://rustup.rs) |
| **Git** | `init`, `status`, `log`, commit proposals | Must be on `PATH` as `git` |
| **Python 3** | `sil parse`, `sil source fetch` | Helpers under `python/`; override with `SIL_PYTHON` |
| **pip packages** (`pypdf`, optional **marker-pdf**) | PDF text extraction | `pypdf` is the light fallback; **Marker** is preferred quality. See `python/requirements.txt` |
| **C toolchain** | Building `sil` (bundled SQLite) | Xcode CLT (macOS), `build-essential` (Debian/Ubuntu), MinGW or MSVC (Windows) |
| **LaTeX engine** | `sil build` | Default config uses **tectonic**; also supports `latexmk`, `pdflatex`, `xelatex`, `lualatex` |

| Feature | Works without extra install? |
|---------|------------------------------|
| `sil init` / structure / SQLite | Needs **git** + built `sil` |
| `sil settings` / `sil tui` | Terminal TUI built into `sil` (powered by Ratatui & Crossterm) |
| `sil parse` | Needs **Python 3**; quality improves with **marker-pdf** |
| `sil source fetch` | Needs **Python 3** (stdlib networking) |
| `sil build` | Needs a **LaTeX engine** on `PATH` |

### Install script (macOS / Linux / Windows)

A single shell installer lives in `install/`. It detects the OS, installs missing tools via the local package manager when possible, then compiles and installs `sil`.

```bash
# From the repository root
chmod +x install/install.sh

# Core: git, Rust, Python, pypdf, C tools → cargo install sil
./install/install.sh

# Also install Marker (large) and a LaTeX engine for sil build
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
# macOS:  brew install git python tectonic
# Debian: sudo apt install git python3 python3-pip build-essential
# Then:
pip install -r python/requirements.txt   # pypdf
# pip install marker-pdf                 # optional, heavy
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
sil settings     # or: sil tui

# After upgrading the sil binary, refresh templates / .gitignore
# (preserves config, structure, manuscripts, and custom gitignore rules)
sil init --update

# Drop PDFs into sources/, then parse
cp ~/Downloads/attention.pdf sources/
sil parse sources/attention.pdf
# or: sil parse          # interactive multi-select of unparsed PDFs

# Search parsed literature
sil search "self-attention"

# Inspect state / agent context
sil status
sil context
sil context --paper --agent --skill-paper

# Build the manuscript (requires configured LaTeX engine)
sil build

# Format into conference/journal templates (NeurIPS, ICML, ICLR, IEEE, arXiv)
sil template apply -t neurips

# Sci-Action annotated history
sil log
```

Optional environment:

| Variable | Effect |
|----------|--------|
| `NO_COLOR` / `SIL_NO_COLOR=1` | Disable colors |
| `SIL_NONINTERACTIVE=1` | No spinners/prompts; parse selects all |
| `SIL_PYTHON` | Python executable (default `python3`) |
| `SIL_MARKER_BIN` | Path to pre-installed `marker_single` / `marker` CLI binary |
| `SIL_MARKER_MODE` | Parsing mode for Marker (default `balance`) |
| `SIL_MARKER_FLAGS` | Custom space-separated CLI flags for `marker_single` |
| `SIL_PARSE_SCRIPT` | Path to `parse_with_marker.py` fallback helper |
| `SIL_DOWNLOAD_SCRIPT` | Path to `download_pdf.py` |
| `SIL_MARKER_STUB` | Test-only: skip Marker, use this text |

Python helpers (`python/`) need a working `python3`. Marker is preferred for parse quality; a fallback exists when Marker is not installed. See `python/requirements.txt` and [External dependencies](#external-dependencies).

---

## Command overview

| Command | Description |
|---------|-------------|
| `sil init [name]` | Create full project tree, templates, auto `.gitignore`, git repo, SQLite DB; **propose** first commit |
| `sil init --update` | Upgrade an existing project to the current sil templates (skills, managed `.gitignore`, missing scaffold) |
| `sil dashboard` / `sil daily` | Interactive Ratatui TUI command center dashboard (progress, health audit, literature feed, idea blocks) |
| `sil settings` / `sil tui` | Interactive Ratatui TUI to manage global author requisites, local project settings, and co-author/grant cache |
| `sil status [--json]` | Stage, git status, source counts, structure completion, draft dirty flag |
| `sil digest [query]` | Fetch top peer-reviewed journal publications digest (Crossref API) |
| `sil todo [--json]` | List active `# -- X -- #` idea and TODO blocks parsed from `paper_draft.tex` |
| `sil parse [path]` | Parse PDF (via Marker) or Markdown/Text/HTML sources natively into SQLite + FTS5 |
| `sil source fetch <doi\|arxiv\|url>` | Download PDF, HTML, or Markdown into `sources/` via DOI (`10.xxxx`), arXiv ID, or URL |
| `sil source list [--json]` | List sources with format tags (`[pdf/parsed/on-disk]`, `[md/parsed/on-disk]`), metadata, and visibility |
| `sil source remove <id>` | Drop a source from the DB so it can be reparsed |
| `sil search <query>` | FTS5 full-text search over parsed sources |
| `sil build [release]` | Compile `config.latex.main` with `config.latex.engine` (`release` mode applies target template, strips `#-- X --#` draft notes, and generates an autonomous journal submission `.zip` archive) |
| `sil template list\|apply` | Collect draft prose into ML/AI templates (`neurips`, `icml`, `iclr`, `ieee`, `arxiv`, `standard`) |
| `sil log` | Git log filtered/annotated by `Sci-Action` trailers |
| `sil context [flags]` | Structured context dump for humans/agents |
| `sil split` | Write agent-readable section files under `.sil/draft_sections/` (does not edit `paper_draft.tex`) |
| `sil propose [--action …]` | Print a Sci-Action commit proposal from dirty paths or an explicit action (never commits) |
| `sil promote [--force]` | Copy `paper_draft.tex` → `paper.tex` and propose `promote-to-final` |
| `sil structure list\|set` | Inspect or update section completion in `structure.yaml` |
| `sil cite <source\|query>` | Suggest BibTeX + `\cite{…}` incorporating stored authors, year, venue, and DOI (optional `--append` to `references.bib`) |
| `sil doctor [--json]` | Project layout, host dependencies, and manuscript health audit (citations, labels, word count) |
| `sil mcp [--quiet]` | Start stdio Model Context Protocol (MCP) JSON-RPC server for AI assistants (Antigravity, Claude Desktop, Cursor) |


Commit proposals always include a trailer such as:

```text
Sci-Action: init
Sci-Action: parse-pdf
Sci-Action: fetch-source
```

`sil` **never** auto-commits.

---

## Settings & Co-Author Cache TUI (`sil settings`)

`sil settings` (or `sil tui`) opens an interactive Ratatui interface for managing:

1. **Global Settings (`~/.config/sil/settings.yaml`)**:
   - Primary Author Name, Email, Affiliation, and ORCID iD.
   - Default Grant Funder, Grant Number, and Acknowledgment prose.
   - Default LaTeX engine and default target template.
2. **Local Project Settings (`.sil/config.yaml`)**:
   - Article title, active co-authors list, active grant requisites, and project notes.
3. **Co-Authors & Grants Cache (`~/.config/sil/cache.yaml`)**:
   - History of all co-authors and grants encountered across previous works.
   - One-key import into local project settings (`u`), fast picker modal (`a`), and deduplication.

### Keybindings in TUI

- `1`-`4` or `Tab` / `Shift+Tab`: Switch tabs (Global, Local, Co-Author Cache, Grant Cache).
- `↑`/`↓` or `j`/`k`: Navigate fields or lists.
- `e` or `Enter`: Edit selected field value.
- `a`: Add new item or select from cache modal.
- `d` / `Delete`: Remove item from local project or cache.
- `u`: Use selected cached item in active local project settings.
- `s` or `Ctrl+S`: Save global settings, local settings, and cache.
- `q` or `Esc`: Quit settings TUI.

---

## Model Context Protocol (MCP) Server & Local ONNX RAG

`sil` provides a native **Model Context Protocol (MCP)** stdio JSON-RPC server (`sil mcp`) allowing external AI IDEs and assistants (such as Antigravity, Claude Desktop, and Cursor) to directly inspect literature, execute skills, update `# -- X -- #` TODO blocks, fetch literature sources, and format commit proposals (12 core tools).

### Key MCP Features

1. **100% Local ONNX Hybrid RAG (`sil_search_sources`)**:
   - **Parent-Child Chunking**: Splits parsed Markdown literature by section headings (parent chunks) and paragraphs (child chunks).
   - **Dense ONNX Embeddings + BM25 FTS5**: Uses local ONNX models (`bge-small-en-v1.5` / `ms-marco-MiniLM-L-6-v2`) with Reciprocal Rank Fusion (RRF) and HyDE query expansion.
   - **Custom ONNX Paths**: Configure custom model directories (`onnx_models_dir`) or explicit file paths (`onnx_embedder_path`, `onnx_reranker_path`) in `.sil/config.yaml` or `~/.config/sil/settings.yaml`.
   - **Parent Context Expansion**: Matches on child paragraphs automatically expand to full parent section context.
2. **Structured Async TODO Governance (`sil_list_todos`, `sil_update_todo`)**:
   - Query, prioritize, and update `% # -- X -- #` comment blocks inside `paper_draft.tex` with status (`open`, `in_progress`, `resolved`), priority (`low`, `medium`, `high`, `critical`), section tags, and author provenance.
3. **Literature Fetching (`sil_fetch_source`)**:
   - Download papers/sources into `sources/` by DOI (`10.xxxx`), arXiv ID (`arxiv:XXXX.YYYY`), or direct URL, and optionally parse into SQLite FTS5 index.
4. **Commit Proposal Governance (`sil_propose_commit`)**:
   - Generates structured commit proposals with `Sci-Action:` trailers for human review. **Never auto-commits**.

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

---

## Layout created by `sil init`

```text
my-paper/
├── .sil/
│   ├── config.yaml          # project paths, latex engine, local settings (co-authors, grants)
│   ├── structure.yaml
│   ├── structure.example.yaml
│   ├── db.sqlite
│   ├── draft_sections/      # agent section cache from `sil split`
│   ├── improvement/         # suggestion_n improvement proposals (tracked)
│   └── skills/
│       ├── SYSTEM.md
│       ├── paper.md
│       └── agent-code.md
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
├── agent/                   # agent-written helper code
│   └── README.md
└── README.md
```

### Folder purposes

- **`sources/`** — original scientific PDFs. Nothing else. Parsed content lives only in SQLite.  
- **`data/`** — experimental / collected data; document each file in `data/README.md`.  
- **`figures/plots/`** — plots from code; list script + figure ref in the README.  
- **`figures/images/`** — external images; document origin and license.  
- **`agent/`** — helper scripts the agent writes; document purpose and how to run.  
- **`.sil/draft_sections/`** — deterministic per-section split of `paper_draft.tex` for agents (`sil split`); do not edit as source of truth.  
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
| `.sil/skills/*` | Folder READMEs, layout dirs | `.sil/config.yaml` |
| `.sil/structure.example.yaml` | Paper stubs, `references.bib` | `.sil/structure.yaml` |
| sil-managed `.gitignore` block | Project `README.md` | `paper_draft.tex`, `paper.tex` |
| | SQLite DB / git repo (ensured) | Custom gitignore rules outside the managed block |

Proposes a commit with `Sci-Action: update` (never auto-committed).

---

## How agents should interact

1. Read `.sil/skills/SYSTEM.md` first (always loaded by `sil context`).  
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
- `sil dashboard` / `sil daily` (TUI Command Center)
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
  sil-core/     # domain types, Config, settings, errors, paths, terminal UX
  sil-db/       # SQLite + FTS5
  sil-git/      # status, commit proposals, Sci-Action trailers
  sil-parse/    # PDF validation, Marker orchestration, native Crossref metadata hydration
  sil-regex/    # centralized regular expressions and pattern matchers
  sil-latex/    # engine abstraction + section splitter
  sil-agent/    # dynamic skills + context generation
  sil-template/ # ML/AI conference article templates (NeurIPS, ICML, ICLR, IEEE, arXiv)
  sil-tui/      # Ratatui TUI for global/local settings and co-author cache
python/         # download_pdf.py, parse_with_marker.py
templates/      # files copied by sil init
docs/adr/       # Architectural Decision Records (ADRs)
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
| Rich BibTeX generation (`sil cite`) | Done |
| Marker via Python helper (stubbable for tests) | Done |
| Commit proposals + `sil log` Sci-Action trailers | Done |
| `sil build` / `sil source fetch` / `sil context` + skills | Done |
| `sil template list\|apply` ML/AI manuscript templating | Done |
| Colored output + progress (disabled in tests) | Done |
| Auto-commit | **Never** (by design) |
| Engines beyond Marker | Out of scope for MVP |

---

## License

MIT
