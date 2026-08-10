# Manuscript estimate & multi-perspective review (sil)

You are reviewing a **sil**-managed scientific manuscript. This skill teaches a multi-perspective estimate process. It is **inspired by** [Academic Research Skills `academic-paper-reviewer`](https://github.com/Imbad0202/academic-research-skills) (CC-BY-NC 4.0) but is a **sil-native** implementation — do not copy external agent files.

## Iron rules

1. **Read-only on the manuscript.** Never edit `paper_draft.tex` or `paper.tex` while estimating. Write only under `.sil/reviews/` or `.sil/improvement/`.
2. **Never auto-commit.** Use Sci-Action proposals (`estimate-paper`).
3. **No fabricated findings.** Every issue must cite a location (section, line band, or structure id).
4. **Devil’s Advocate CRITICAL** items must appear explicitly; they block a silent “accept” summary.
5. Scores are **advisory**, not peer-review truth. Label L0 (heuristic) vs L1 (your LLM panel).

## Modes

| Mode | When | Depth |
|------|------|--------|
| `quick` | Fast quality snapshot | Structure, health, length, cites |
| `full` | Pre-submission panel | All personas + synthesis |
| `methodology` | Methods-focused | Design, eval, reproducibility |

CLI: `sil paper estimate --mode quick|full|methodology [--json] [--write]`  
MCP: `sil_review` (`action=estimate`)  
Native L0 always runs offline; refine with this skill (L1) when a host model is available.

## Panel personas (L1)

Configure five non-overlapping lenses (do not cross-copy findings):

1. **Journal-Fit** — venue fit, significance, audience  
2. **Methodology** — design, statistics, evaluation protocol  
3. **Domain** — literature coverage, missing key refs  
4. **Perspective** — cross-disciplinary impact, ethics  
5. **Devil’s Advocate** — strongest counter-argument, CRITICAL gaps  

Then **synthesize**: consensus vs disagreement → decision + revision roadmap.

Decision map (overall 0–100): ≥80 accept · 65–79 minor_revision · 50–64 major_revision · <50 reject.

## Rubrics & templates

- Dimension rubrics: `agent/skills/review/rubrics.md`  
- Persona prompts: `agent/skills/review/personas.md`  
- Report skeleton: `agent/skills/review/report_template.md`  

Prefer emitting JSON matching sil’s estimate schema (`schema_version: 1`) plus a human `report.md`.

## sil project paths

- Draft: `paper_draft.tex` (source of truth for prose)  
- Structure: `.sil/structure.yaml`  
- Bibliography: `references.bib`  
- Reports: `.sil/reviews/review_*/`  
- Improvements: `.sil/improvement/suggestion_n`  
- Context: `sil project context --skill review.md --paper`  
- Health: `sil project doctor` / manuscript audit signals  

## Workflow

1. Run `sil paper estimate --mode full --json` for L0 baseline.  
2. Load this skill + draft sections (`.sil/draft_sections/` or `sil paper split`).  
3. Run L1 panel; adjudicate DA CRITICAL.  
4. Write refined report under `.sil/reviews/` (or ask user to re-run with agent-written JSON).  
5. Optionally file roadmap as `.sil/improvement/suggestion_n`.  
6. Propose commit with `Sci-Action: estimate-paper` — human commits.

## Attribution

Based on methodology patterns from Academic Research Skills by Cheng-I Wu  
https://github.com/Imbad0202/academic-research-skills  
License of upstream materials: CC-BY-NC 4.0. This skill text is original sil content.
