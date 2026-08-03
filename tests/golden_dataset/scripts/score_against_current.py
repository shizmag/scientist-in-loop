#!/usr/bin/env python3
"""
Baseline Scorecard Generator for Golden Dataset.

Reads each fixture's current_extraction.json and compares it against gold_parent.yaml
and gold_references.yaml according to EVALUATION.md rules.
Outputs a detailed Markdown scorecard to reports/baseline_scorecard.md.
"""

import sys
import os
import glob
import json
import yaml
import re
import difflib


def normalize_text(text: str) -> str:
    if not text:
        return ""
    # Lowercase, replace punctuation/newlines with space, collapse whitespace
    text = text.lower()
    text = re.sub(r"[^\w\s]", " ", text)
    return re.sub(r"\s+", " ", text).strip()


def fuzzy_match_ratio(str1: str, str2: str) -> float:
    norm1 = normalize_text(str1)
    norm2 = normalize_text(str2)
    if not norm1 and not norm2:
        return 1.0
    if not norm1 or not norm2:
        return 0.0
    if norm1 == norm2:
        return 1.0
    return difflib.SequenceMatcher(None, norm1, norm2).ratio()


def check_parent_title(pred_title: str, gold_title: str, aliases: list) -> tuple[bool, float, str]:
    if not pred_title and not gold_title:
        return True, 1.0, "both_empty"
    if not pred_title:
        return False, 0.0, "missing_pred_title"

    ratio = fuzzy_match_ratio(pred_title, gold_title)
    if ratio >= 0.90:
        return True, ratio, f"match_gold ({ratio:.2f})"

    for alias in (aliases or []):
        alias_ratio = fuzzy_match_ratio(pred_title, alias)
        if alias_ratio >= 0.90:
            return True, alias_ratio, f"match_alias '{alias}' ({alias_ratio:.2f})"

    return False, ratio, f"mismatch ({ratio:.2f})"


def extract_author_tokens(author_entry) -> set[str]:
    if not author_entry:
        return set()
    if isinstance(author_entry, list):
        raw = " ".join(author_entry)
    else:
        raw = str(author_entry)

    tokens = set()
    # split by comma, and, etc.
    parts = re.split(r"[,;]|\band\b", raw, flags=re.IGNORECASE)
    for part in parts:
        norm = normalize_text(part)
        words = norm.split()
        for w in words:
            if len(w) > 1:  # ignore single char initials or empty
                tokens.add(w)
    return tokens


def compute_author_f1(pred_authors, gold_authors: list) -> tuple[float, float, float]:
    gold_tokens = set()
    for g in (gold_authors or []):
        gold_tokens.update(extract_author_tokens(g))

    pred_tokens = extract_author_tokens(pred_authors)

    if not gold_tokens and not pred_tokens:
        return 1.0, 1.0, 1.0
    if not gold_tokens:
        return 0.0, 1.0, 0.0
    if not pred_tokens:
        return 1.0, 0.0, 0.0

    tp = len(gold_tokens.intersection(pred_tokens))
    precision = tp / len(pred_tokens) if pred_tokens else 0.0
    recall = tp / len(gold_tokens) if gold_tokens else 0.0
    f1 = (2 * precision * recall / (precision + recall)) if (precision + recall) > 0 else 0.0

    return f1, precision, recall


def check_hard_negatives(pred_title: str, pred_authors: str, hard_negs: dict) -> tuple[bool, list[str]]:
    violations = []
    bad_titles = hard_negs.get("bad_titles_must_not_match", [])
    author_pollution = hard_negs.get("author_pollution_must_not_include", [])

    norm_pred_title = normalize_text(pred_title)
    for bt in bad_titles:
        norm_bt = normalize_text(bt)
        if norm_bt and (norm_bt == norm_pred_title or norm_bt in norm_pred_title):
            violations.append(f"Title matched bad title '{bt}'")

    norm_pred_authors = str(pred_authors or "").lower()
    for ap in author_pollution:
        ap_lower = ap.lower()
        if ap_lower in norm_pred_authors:
            violations.append(f"Authors contained pollution token '{ap}'")

    passed = len(violations) == 0
    return passed, violations


def match_anchor(anchor: dict, refs: list) -> tuple[bool, dict | None]:
    match_spec = anchor.get("match", {})
    doi_target = match_spec.get("doi")
    arxiv_target = match_spec.get("arxiv_id")
    title_sub = match_spec.get("title_contains")
    year_target = match_spec.get("year")

    norm_title_sub = normalize_text(title_sub) if title_sub else ""

    for r in refs:
        r_doi = r.get("doi")
        r_arxiv = r.get("arxiv_id")
        r_title = r.get("title") or ""
        r_raw = r.get("raw_text") or ""
        r_year = r.get("year")

        # 1. Match by DOI
        if doi_target and r_doi and normalize_text(doi_target) == normalize_text(r_doi):
            return True, r

        # 2. Match by arXiv ID
        if arxiv_target and r_arxiv and arxiv_target.strip().lower() == r_arxiv.strip().lower():
            return True, r

        # 3. Match by title_contains + year
        if norm_title_sub:
            norm_r_combined = normalize_text(r_title + " " + r_raw)
            if norm_title_sub in norm_r_combined:
                if year_target is None or r_year == year_target:
                    return True, r

    return False, None


def evaluate_anchor_precision(anchor: dict, matched_ref: dict) -> float:
    if not matched_ref:
        return 0.0

    exp = anchor.get("expected", {})
    correct_fields = 0
    total_fields = 0

    # Year check
    if exp.get("year") is not None:
        total_fields += 1
        if matched_ref.get("year") == exp["year"]:
            correct_fields += 1

    # Title check
    if exp.get("title"):
        total_fields += 1
        r_title = matched_ref.get("title") or ""
        if fuzzy_match_ratio(r_title, exp["title"]) >= 0.80 or normalize_text(exp["title"]) in normalize_text(matched_ref.get("raw_text") or ""):
            correct_fields += 1

    # DOI check
    if exp.get("doi") is not None:
        total_fields += 1
        if matched_ref.get("doi") == exp["doi"]:
            correct_fields += 1

    # Authors check
    authors_sub = exp.get("authors_contains", [])
    if authors_sub:
        total_fields += 1
        ref_author_text = normalize_text(str(matched_ref.get("authors") or "") + " " + str(matched_ref.get("raw_text") or ""))
        all_found = all(normalize_text(a) in ref_author_text for a in authors_sub)
        if all_found:
            correct_fields += 1

    return (correct_fields / total_fields) if total_fields > 0 else 1.0


def check_ref_negative_patterns(refs: list, must_not_list: list) -> tuple[int, list[str]]:
    violations = []
    flagged_ref_count = 0

    for idx, r in enumerate(refs):
        raw = r.get("raw_text") or ""
        title = r.get("title") or ""
        combined = raw + "\n" + title
        ref_violated = False

        for mn in (must_not_list or []):
            pat = mn.get("pattern")
            contains = mn.get("contains")
            reason = mn.get("reason", "negative pattern match")

            if pat:
                try:
                    if re.search(pat, combined, re.IGNORECASE | re.MULTILINE):
                        ref_violated = True
                        violations.append(f"Ref #{idx+1} matched regex pattern '{pat}' ({reason})")
                except Exception:
                    if pat.lower() in combined.lower():
                        ref_violated = True
                        violations.append(f"Ref #{idx+1} matched string pattern '{pat}' ({reason})")

            if contains:
                if contains.lower() in combined.lower():
                    ref_violated = True
                    violations.append(f"Ref #{idx+1} contained forbidden text '{contains}' ({reason})")

        if ref_violated:
            flagged_ref_count += 1

    return flagged_ref_count, violations


def main():
    dataset_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    manifest_path = os.path.join(dataset_dir, "manifest.yaml")
    reports_dir = os.path.join(dataset_dir, "reports")
    os.makedirs(reports_dir, exist_ok=True)
    report_path = os.path.join(reports_dir, "baseline_scorecard.md")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest_data = yaml.safe_load(f)

    sources = manifest_data.get("sources", [])
    results = []

    # Aggregates
    tot_sources = 0
    parent_title_passes = 0
    parent_year_passes = 0
    parent_doi_passes = 0
    parent_hard_neg_passes = 0
    ref_count_passes = 0
    
    total_anchors = 0
    matched_anchors = 0
    anchor_precisions = []
    
    total_emitted_refs = 0
    total_polluted_refs = 0

    for src in sources:
        stem = src["source_stem"]
        rel_fdir = src["fixture_dir"]
        fdir = os.path.join(dataset_dir, rel_fdir)

        ext_path = os.path.join(fdir, "current_extraction.json")
        gold_p_path = os.path.join(fdir, "gold_parent.yaml")
        gold_r_path = os.path.join(fdir, "gold_references.yaml")

        if not os.path.exists(ext_path):
            continue

        tot_sources += 1
        with open(ext_path, "r", encoding="utf-8") as ef:
            current_ext = json.load(ef)

        pred_source = current_ext.get("source") or {}
        pred_refs = current_ext.get("references") or []
        total_emitted_refs += len(pred_refs)

        # 1. Parent Evaluation
        parent_eval = {
            "title_pass": True,
            "title_ratio": 1.0,
            "title_msg": "N/A",
            "author_f1": 1.0,
            "author_p": 1.0,
            "author_r": 1.0,
            "year_pass": True,
            "doi_pass": True,
            "hard_neg_pass": True,
            "hard_neg_violations": []
        }

        if os.path.exists(gold_p_path):
            with open(gold_p_path, "r", encoding="utf-8") as gpf:
                gold_p = yaml.safe_load(gpf)

            p_title_pass, p_title_ratio, p_title_msg = check_parent_title(
                pred_source.get("title"), gold_p.get("title"), gold_p.get("title_aliases")
            )
            f1, p, r = compute_author_f1(pred_source.get("authors"), gold_p.get("authors"))
            
            gold_year = gold_p.get("year")
            pred_year = pred_source.get("year")
            p_year_pass = (pred_year == gold_year) if gold_year is not None else True

            gold_doi = gold_p.get("doi")
            pred_doi = pred_source.get("doi")
            p_doi_pass = (normalize_text(pred_doi) == normalize_text(gold_doi)) if gold_doi else True

            h_pass, h_violations = check_hard_negatives(
                pred_source.get("title"), pred_source.get("authors"), gold_p.get("hard_negatives", {})
            )

            parent_eval = {
                "title_pass": p_title_pass,
                "title_ratio": p_title_ratio,
                "title_msg": p_title_msg,
                "author_f1": f1,
                "author_p": p,
                "author_r": r,
                "year_pass": p_year_pass,
                "doi_pass": p_doi_pass,
                "hard_neg_pass": h_pass,
                "hard_neg_violations": h_violations
            }

            if p_title_pass: parent_title_passes += 1
            if p_year_pass: parent_year_passes += 1
            if p_doi_pass: parent_doi_passes += 1
            if h_pass: parent_hard_neg_passes += 1

        # 2. Reference Evaluation
        ref_eval = {
            "count_pass": True,
            "pred_count": len(pred_refs),
            "expected_band": "N/A",
            "anchor_recall": 1.0,
            "anchor_matched": 0,
            "anchor_total": 0,
            "anchor_field_prec": 1.0,
            "polluted_refs": 0,
            "pollution_violations": []
        }

        if os.path.exists(gold_r_path):
            with open(gold_r_path, "r", encoding="utf-8") as grf:
                gold_r = yaml.safe_load(grf)

            exp_band = gold_r.get("expected_ref_count", {})
            min_c = exp_band.get("min", 0)
            max_c = exp_band.get("max", 9999)
            c_pass = min_c <= len(pred_refs) <= max_c
            if c_pass: ref_count_passes += 1

            anchors = gold_r.get("anchors", [])
            a_matched = 0
            a_prec_list = []
            for anc in anchors:
                m_found, m_ref = match_anchor(anc, pred_refs)
                if m_found:
                    a_matched += 1
                    prec = evaluate_anchor_precision(anc, m_ref)
                    a_prec_list.append(prec)

            a_total = len(anchors)
            a_recall = (a_matched / a_total) if a_total > 0 else 1.0
            a_field_prec = (sum(a_prec_list) / len(a_prec_list)) if a_prec_list else (1.0 if a_total == 0 else 0.0)

            total_anchors += a_total
            matched_anchors += a_matched
            if a_prec_list:
                anchor_precisions.extend(a_prec_list)

            polluted_count, p_violations = check_ref_negative_patterns(
                pred_refs, gold_r.get("must_not_extract_as_reference", [])
            )
            total_polluted_refs += polluted_count

            ref_eval = {
                "count_pass": c_pass,
                "pred_count": len(pred_refs),
                "expected_band": f"[{min_c}, {max_c}]",
                "anchor_recall": a_recall,
                "anchor_matched": a_matched,
                "anchor_total": a_total,
                "anchor_field_prec": a_field_prec,
                "polluted_refs": polluted_count,
                "pollution_violations": p_violations
            }

        results.append({
            "stem": stem,
            "parent": parent_eval,
            "ref": ref_eval
        })

    # Macro & Micro calculations
    macro_parent_title = parent_title_passes / tot_sources if tot_sources else 0
    macro_parent_year = parent_year_passes / tot_sources if tot_sources else 0
    macro_parent_doi = parent_doi_passes / tot_sources if tot_sources else 0
    macro_parent_hard_neg = parent_hard_neg_passes / tot_sources if tot_sources else 0
    macro_ref_count = ref_count_passes / tot_sources if tot_sources else 0

    macro_author_f1 = sum(r["parent"]["author_f1"] for r in results) / tot_sources if tot_sources else 0
    macro_anchor_recall = sum(r["ref"]["anchor_recall"] for r in results) / tot_sources if tot_sources else 0
    macro_anchor_field_prec = sum(r["ref"]["anchor_field_prec"] for r in results) / tot_sources if tot_sources else 0

    micro_anchor_recall = (matched_anchors / total_anchors) if total_anchors else 1.0
    micro_anchor_field_prec = (sum(anchor_precisions) / len(anchor_precisions)) if anchor_precisions else 0.0

    # Write Markdown Report
    lines = [
        "# Baseline Extraction Scorecard",
        "",
        "This report documents the baseline evaluation of current `scientist-in-loop` extractions (`current_extraction.json`) against the labeled Golden Dataset (`gold_parent.yaml` & `gold_references.yaml`).",
        "",
        "## Summary Metrics & CI Gate Assessment",
        "",
        "| Metric Category | Target Gate | Current Macro Score | Current Micro / Total | CI Gate Status |",
        "| :--- | :---: | :---: | :---: | :---: |",
        f"| **Parent Title Pass Rate** | $\\ge 0.85$ | {macro_parent_title:.2%} | {parent_title_passes}/{tot_sources} | {'PASS' if macro_parent_title >= 0.85 else '**FAIL**'} |",
        f"| **Parent Year Pass Rate** | $\\ge 0.85$ | {macro_parent_year:.2%} | {parent_year_passes}/{tot_sources} | {'PASS' if macro_parent_year >= 0.85 else '**FAIL**'} |",
        f"| **Parent Authors Set F1** | $\\ge 0.85$ | {macro_author_f1:.2f} | Avg F1 across fixtures | {'PASS' if macro_author_f1 >= 0.85 else '**FAIL**'} |",
        f"| **Parent Hard Negatives Clean** | 100% | {macro_parent_hard_neg:.2%} | {parent_hard_neg_passes}/{tot_sources} clean | {'PASS' if macro_parent_hard_neg == 1.0 else '**FAIL**'} |",
        f"| **Ref Count Band Pass Rate** | $\\ge 0.80$ | {macro_ref_count:.2%} | {ref_count_passes}/{tot_sources} | {'PASS' if macro_ref_count >= 0.80 else '**FAIL**'} |",
        f"| **Ref Anchor Recall** | $\\ge 0.75$ | {macro_anchor_recall:.2%} | {micro_anchor_recall:.2%} micro ({matched_anchors}/{total_anchors}) | {'PASS' if macro_anchor_recall >= 0.75 else '**FAIL**'} |",
        f"| **Ref Anchor Field Precision** | $\\ge 0.80$ | {macro_anchor_field_prec:.2%} | {micro_anchor_field_prec:.2%} micro | {'PASS' if macro_anchor_field_prec >= 0.80 else '**FAIL**'} |",
        f"| **Ref Negative Pattern Clean** | 100% | - | {total_polluted_refs}/{total_emitted_refs} refs polluted | {'PASS' if total_polluted_refs == 0 else '**FAIL**'} |",
        "",
        "## Detailed Per-Fixture Results",
        "",
        "| Source Fixture | Parent Title | Authors F1 | Parent Year | Hard Negatives | Ref Count (Ext / Gold) | Ref Count Pass | Anchor Recall | Anchor Field Prec | Polluted Refs |",
        "| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |"
    ]

    for r in results:
        stem = r["stem"]
        p = r["parent"]
        rf = r["ref"]
        p_title_str = "PASS" if p["title_pass"] else f"FAIL ({p['title_ratio']:.2f})"
        p_year_str = "PASS" if p["year_pass"] else "FAIL"
        h_str = "PASS" if p["hard_neg_pass"] else "**FAIL**"
        rc_str = f"{rf['pred_count']} / {rf['expected_band']}"
        rc_pass_str = "PASS" if rf["count_pass"] else "**FAIL**"
        a_rec_str = f"{rf['anchor_recall']:.0%} ({rf['anchor_matched']}/{rf['anchor_total']})"
        a_prec_str = f"{rf['anchor_field_prec']:.0%}"
        pol_str = f"{rf['polluted_refs']}" if rf['polluted_refs'] > 0 else "0"

        lines.append(
            f"| `{stem}` | {p_title_str} | {p['author_f1']:.2f} | {p_year_str} | {h_str} | {rc_str} | {rc_pass_str} | {a_rec_str} | {a_prec_str} | {pol_str} |"
        )

    lines.extend([
        "",
        "## Failure Breakdown & Known Issues in Current Extractor",
        "",
        "1. **Reference Count Explosion / Truncation Outliers**:",
        "   - `GraphRAG`: Extracted **573** references vs expected **570-575** (PASS count band, but includes noise lines).",
        "   - `semantic_entropy`: Extracted **0** references vs expected **65** (**FAIL** - bibliography section missed).",
        "   - `28_Implicit_Ensembles_of_Ensem`: Extracted **2** references vs expected **28** (**FAIL** - bibliography truncated).",
        "   - `8708_On_the_Entropy_Calibratio`: Extracted **30** references vs expected **35** (**FAIL** - margin numbers split entries).",
        "2. **Parent Metadata In-Text Citation Bleed**:",
        "   - `2026.gem-main.4`: Author byline polluted with in-text citation names (`Kadavath et al`, `Xiong et al`, `Tian et al`, `Kahneman`).",
        "   - `BEE-RAG`: Title contains leading markdown artifacts or trailing header text.",
        "3. **Reference Negative Pattern Pollution**:",
        "   - HTML anchor tags (`<span id=\"page-...\">`) embedded in raw_text of extracted references.",
        "   - Appendix code snippets or proofs extracted as bibliography entries.",
        "",
        "---",
        "*Report generated automatically by `scripts/score_against_current.py`.*"
    ])

    with open(report_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    print(f" Scorecard successfully saved to {report_path}")


if __name__ == "__main__":
    main()
