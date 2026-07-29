# scientist-in-loop (`sil`)

**scientist-in-loop** is a Rust CLI that turns a folder containing a scientific paper into a self-documenting, agent-friendly workspace.

Humans and LLM agents work under the same strict conventions:

- **Git** is the single source of history  
- **SQLite + FTS5** is the memory for parsed sources  
- **`structure.yaml`** is the formal syntactic map of the paper  
- **`paper_draft.tex`** is the working document (later promoted to `paper.tex`)  
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

1. Original PDFs live only in `sources/`.  
2. Parsed text is indexed in SQLite/FTS5 — never duplicated as loose markdown dumps.  
3. The high-level plan lives in `.sil/structure.yaml` with explicit section completion.  
4. Every meaningful change produces a **commit proposal** with a `Sci-Action:` trailer — never an auto-commit.  
5. Agents load `SYSTEM.md` always, and `paper.md` / `agent-code.md` only when the task touches those surfaces.

---

## Quick start

```bash
# From this repository
cargo install --path crates/sil

# Create a project
sil init my-paper
cd my-paper

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

# Sci-Action annotated history
sil log
```

Optional environment:

| Variable | Effect |
|----------|--------|
| `NO_COLOR` / `SIL_NO_COLOR=1` | Disable colors |
| `SIL_NONINTERACTIVE=1` | No spinners/prompts; parse selects all |
| `SIL_PYTHON` | Python executable (default `python3`) |
| `SIL_PARSE_SCRIPT` | Path to `parse_with_marker.py` |
| `SIL_DOWNLOAD_SCRIPT` | Path to `download_pdf.py` |
| `SIL_MARKER_STUB` | Test-only: skip Marker, use this text |

Python helpers (`python/`) need a working `python3`. Marker is preferred for parse quality; a fallback exists when Marker is not installed. See `python/requirements.txt`.

---

## Command overview

| Command | Description |
|---------|-------------|
| `sil init [name]` | Create full project tree, templates, git repo, SQLite DB; **propose** first commit |
| `sil status` | Stage, git status, source counts, structure completion, draft dirty flag |
| `sil parse [pdf]` | Parse one PDF, or interactively multi-select unparsed files in `sources/` |
| `sil source fetch <doi\|arxiv\|url>` | Download PDF into `sources/`, offer parse |
| `sil search <query>` | FTS5 full-text search over parsed sources |
| `sil build` | Compile `config.latex.main` with `config.latex.engine` |
| `sil log` | Git log filtered/annotated by `Sci-Action` trailers |
| `sil context [flags]` | Structured context dump for humans/agents |

Commit proposals always include a trailer such as:

```text
Sci-Action: init
Sci-Action: parse-pdf
Sci-Action: fetch-source
```

`sil` **never** auto-commits.

---

## Layout created by `sil init`

```text
my-paper/
├── .sil/
│   ├── config.yaml
│   ├── structure.yaml
│   ├── structure.example.yaml
│   ├── db.sqlite
│   └── skills/
│       ├── SYSTEM.md
│       ├── paper.md
│       └── agent-code.md
├── paper_draft.tex
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
7. Never invent new top-level directories. Never auto-commit.

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
  sil-core/     # domain types, Config, errors, paths, terminal UX
  sil-db/       # SQLite + FTS5
  sil-git/      # status, commit proposals, Sci-Action trailers
  sil-parse/    # PDF validation + Marker orchestration
  sil-latex/    # engine abstraction + section splitter
  sil-agent/    # dynamic skills + context generation
python/         # download_pdf.py, parse_with_marker.py
templates/      # files copied by sil init
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
| `sil init` exact layout + templates + git + SQLite | Done |
| Typed `config.yaml` / `structure.yaml` + `sil status` | Done |
| `sil parse` (path + noninteractive multi-select) + FTS5 `sil search` | Done |
| Marker via Python helper (stubbable for tests) | Done |
| Commit proposals + `sil log` Sci-Action trailers | Done |
| `sil build` / `sil source fetch` / `sil context` + skills | Done |
| Colored output + progress (disabled in tests) | Done |
| Auto-commit | **Never** (by design) |
| Engines beyond Marker | Out of scope for MVP |

---

## License

MIT
