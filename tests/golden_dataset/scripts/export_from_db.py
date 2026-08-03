#!/usr/bin/env python3
import os
import sys
import json
import sqlite3
import hashlib
import argparse

DEFAULT_DB_PATH = "/Users/vladimirkasterin/articles/entropy_framework/.sil/db.sqlite"
DEFAULT_PDF_DIR = "/Users/vladimirkasterin/articles/entropy_framework/sources"
DEFAULT_OUTPUT_DIR = "/Users/vladimirkasterin/rust/scientist-in-loop/tests/golden_dataset"


def sanitize_stem(filename: str) -> str:
    name = filename
    if name.lower().endswith(".pdf"):
        name = name[:-4]
    return name.replace(" ", "_")


def to_yaml(data, indent=0) -> str:
    lines = []
    ind = " " * indent
    if isinstance(data, dict):
        for k, v in data.items():
            if isinstance(v, (dict, list)):
                lines.append(f"{ind}{k}:")
                lines.append(to_yaml(v, indent + 2))
            elif v is None:
                lines.append(f"{ind}{k}: null")
            elif isinstance(v, bool):
                lines.append(f"{ind}{k}: {'true' if v else 'false'}")
            elif isinstance(v, (int, float)):
                lines.append(f"{ind}{k}: {v}")
            else:
                v_str = str(v)
                if "\n" in v_str:
                    lines.append(f"{ind}{k}: |")
                    for line in v_str.splitlines():
                        lines.append(f"{ind}  {line}")
                elif any(
                    c in v_str
                    for c in [
                        ":",
                        "#",
                        "[",
                        "]",
                        "{",
                        "}",
                        ",",
                        "*",
                        "&",
                        "!",
                        "?",
                        "|",
                        ">",
                        "-",
                        "<",
                        "=",
                        "%",
                        "@",
                        '\\"',
                    ]
                ):
                    lines.append(f"{ind}{k}: {json.dumps(v_str)}")
                else:
                    lines.append(f"{ind}{k}: {v_str}")
    elif isinstance(data, list):
        for item in data:
            if isinstance(item, dict):
                first = True
                for k, v in item.items():
                    if first:
                        if isinstance(v, (dict, list)):
                            lines.append(f"{ind}- {k}:")
                            lines.append(to_yaml(v, indent + 4))
                        elif v is None:
                            lines.append(f"{ind}- {k}: null")
                        elif isinstance(v, bool):
                            lines.append(f"{ind}- {k}: {'true' if v else 'false'}")
                        elif isinstance(v, (int, float)):
                            lines.append(f"{ind}- {k}: {v}")
                        else:
                            v_str = str(v)
                            if "\n" in v_str:
                                lines.append(f"{ind}- {k}: |")
                                for line in v_str.splitlines():
                                    lines.append(f"{ind}    {line}")
                            elif any(
                                c in v_str
                                for c in [
                                    ":",
                                    "#",
                                    "[",
                                    "]",
                                    "{",
                                    "}",
                                    ",",
                                    "*",
                                    "&",
                                    "!",
                                    "?",
                                    "|",
                                    ">",
                                    "-",
                                    "<",
                                    "=",
                                    "%",
                                    "@",
                                    '\\"',
                                ]
                            ):
                                lines.append(f"{ind}- {k}: {json.dumps(v_str)}")
                            else:
                                lines.append(f"{ind}- {k}: {v_str}")
                        first = False
                    else:
                        if isinstance(v, (dict, list)):
                            lines.append(f"{ind}  {k}:")
                            lines.append(to_yaml(v, indent + 4))
                        elif v is None:
                            lines.append(f"{ind}  {k}: null")
                        elif isinstance(v, bool):
                            lines.append(f"{ind}  {k}: {'true' if v else 'false'}")
                        elif isinstance(v, (int, float)):
                            lines.append(f"{ind}  {k}: {v}")
                        else:
                            v_str = str(v)
                            if "\n" in v_str:
                                lines.append(f"{ind}  {k}: |")
                                for line in v_str.splitlines():
                                    lines.append(f"{ind}    {line}")
                            elif any(
                                c in v_str
                                for c in [
                                    ":",
                                    "#",
                                    "[",
                                    "]",
                                    "{",
                                    "}",
                                    ",",
                                    "*",
                                    "&",
                                    "!",
                                    "?",
                                    "|",
                                    ">",
                                    "-",
                                    "<",
                                    "=",
                                    "%",
                                    "@",
                                    '\\"',
                                ]
                            ):
                                lines.append(f"{ind}  {k}: {json.dumps(v_str)}")
                            else:
                                lines.append(f"{ind}  {k}: {v_str}")
            else:
                lines.append(f"{ind}- {json.dumps(item)}")
    return "\n".join(lines)


def export_fixtures(db_path: str, pdf_dir: str, output_dir: str):
    if not os.path.exists(db_path):
        raise FileNotFoundError(f"Database not found at {db_path}")

    fixtures_dir = os.path.join(output_dir, "fixtures")
    os.makedirs(fixtures_dir, exist_ok=True)

    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()

    cur.execute("SELECT * FROM sources;")
    sources_rows = [dict(r) for r in cur.fetchall()]

    manifest_sources = []
    summary_rows = []

    for src in sources_rows:
        filename = src["filename"]
        stem = sanitize_stem(filename)
        fixture_path = os.path.join(fixtures_dir, stem)
        os.makedirs(fixture_path, exist_ok=True)

        content = src["content"] or ""
        references_text = src["references_text"] or ""

        content_bytes = len(content.encode("utf-8"))
        refs_text_bytes = len(references_text.encode("utf-8"))

        content_sha256 = hashlib.sha256(content.encode("utf-8")).hexdigest()

        # Fetch references
        cur.execute(
            "SELECT * FROM source_references WHERE source_id = ? ORDER BY ref_index ASC;",
            (src["id"],),
        )
        refs_rows = [dict(r) for r in cur.fetchall()]
        n_current_refs = len(refs_rows)

        # Write content.md
        with open(
            os.path.join(fixture_path, "content.md"), "w", encoding="utf-8"
        ) as f:
            f.write(content)

        # Write references_block.md
        with open(
            os.path.join(fixture_path, "references_block.md"), "w", encoding="utf-8"
        ) as f:
            f.write(references_text)

        # Extract current parent fields for meta & current_extraction
        db_fields = {
            "id": src["id"],
            "filename": src["filename"],
            "path": src["path"],
            "title": src["title"],
            "authors": src["authors"],
            "year": src["year"],
            "doi": src["doi"],
            "venue": src["venue"],
            "abstract_text": src["abstract_text"],
            "kind": src["kind"],
            "status": src["status"],
            "parsed": src["parsed"],
            "created_at": src["created_at"],
            "updated_at": src["updated_at"],
        }

        # Write meta.yaml
        pdf_path = os.path.join(pdf_dir, filename)
        meta_data = {
            "filename": filename,
            "pdf_path": pdf_path,
            "content_sha256": content_sha256,
            "content_bytes": content_bytes,
            "references_text_bytes": refs_text_bytes,
            "current_db_fields": db_fields,
        }
        with open(
            os.path.join(fixture_path, "meta.yaml"), "w", encoding="utf-8"
        ) as f:
            f.write(to_yaml(meta_data) + "\n")

        # Write current_extraction.json
        extraction_data = {
            "source": db_fields,
            "references": refs_rows,
        }
        with open(
            os.path.join(fixture_path, "current_extraction.json"),
            "w",
            encoding="utf-8",
        ) as f:
            json.dump(extraction_data, f, indent=2, ensure_ascii=False)

        # Flags calculation
        empty_content = content_bytes == 0
        empty_references_text = refs_text_bytes == 0
        ref_count_outlier = (n_current_refs > 150) or (
            not empty_references_text and n_current_refs < 3
        )
        needs_reparse = empty_content and os.path.exists(pdf_path)

        flag_list = []
        if empty_content:
            flag_list.append("empty_content")
        if empty_references_text:
            flag_list.append("empty_references_text")
        if ref_count_outlier:
            flag_list.append("ref_count_outlier")
        if needs_reparse:
            flag_list.append("needs_reparse")

        # Check gold_parent.yaml
        gold_parent_path = os.path.join(fixture_path, "gold_parent.yaml")
        has_gold_parent = os.path.exists(gold_parent_path)
        parent_confidence = None
        if has_gold_parent:
            with open(gold_parent_path, "r", encoding="utf-8") as f:
                for line in f:
                    if line.strip().startswith("label_confidence:"):
                        parent_confidence = line.split(":", 1)[1].strip()

        manifest_sources.append(
            {
                "filename": filename,
                "source_stem": stem,
                "fixture_dir": f"fixtures/{stem}",
                "gold_parent": has_gold_parent,
                "parent_confidence": parent_confidence,
                "content_bytes": content_bytes,
                "references_text_bytes": refs_text_bytes,
                "n_current_refs": n_current_refs,
                "flags": {
                    "empty_content": empty_content,
                    "empty_references_text": empty_references_text,
                    "ref_count_outlier": ref_count_outlier,
                },
                "needs_reparse": needs_reparse,
            }
        )

        content_kb = f"{content_bytes / 1024:.1f} KB"
        refs_kb = f"{refs_text_bytes / 1024:.1f} KB"
        flags_str = ", ".join(flag_list) if flag_list else "-"
        summary_rows.append(
            (filename, content_kb, refs_kb, str(n_current_refs), flags_str)
        )

    # Write manifest.yaml
    manifest_data = {"sources": manifest_sources}
    manifest_path = os.path.join(output_dir, "manifest.yaml")
    with open(manifest_path, "w", encoding="utf-8") as f:
        f.write(to_yaml(manifest_data) + "\n")

    conn.close()

    # Print Summary Table
    print("\nGolden Dataset Export Summary:")
    header = f"{'filename':<38} | {'content_kb':<10} | {'refs_block_kb':<13} | {'n_current_refs':<14} | {'flags':<25}"
    print("-" * len(header))
    print(header)
    print("-" * len(header))
    for row in summary_rows:
        print(
            f"{row[0]:<38} | {row[1]:<10} | {row[2]:<13} | {row[3]:<14} | {row[4]:<25}"
        )
    print("-" * len(header))
    print(f"\nSuccessfully exported {len(sources_rows)} sources to {output_dir}\n")


def main():
    parser = argparse.ArgumentParser(
        description="Export golden dataset fixture pack from SQLite database."
    )
    parser.add_argument(
        "--db-path",
        default=DEFAULT_DB_PATH,
        help="Path to SQLite db.sqlite file",
    )
    parser.add_argument(
        "--pdf-dir",
        default=DEFAULT_PDF_DIR,
        help="Path to directory containing raw PDFs",
    )
    parser.add_argument(
        "--output-dir",
        default=DEFAULT_OUTPUT_DIR,
        help="Path to output root directory for golden dataset",
    )

    args = parser.parse_args()
    export_fixtures(args.db_path, args.pdf_dir, args.output_dir)


if __name__ == "__main__":
    main()
