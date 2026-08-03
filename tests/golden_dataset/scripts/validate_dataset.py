#!/usr/bin/env python3
"""
Dataset Integrity Validator for golden_dataset.

Checks:
1. YAML syntax parsing across all .yaml / .yml files.
2. Manifest structure against schema/manifest.schema.json.
3. For each manifest entry:
   - Presence of fixture_dir, content.md, meta.yaml.
   - Presence of gold_parent.yaml / gold_references.yaml if flagged true in manifest.
   - content_sha256 in meta.yaml matches actual content.md SHA-256 hash.
4. Fixture files against JSON Schemas (schema/gold_parent.schema.json, schema/gold_references.schema.json).
"""

import sys
import os
import glob
import json
import hashlib

try:
    import yaml
except ImportError:
    print(
        "Error: PyYAML is required. From the repo root run:\n"
        "  uv sync --group dev\n"
        "  uv run tests/golden_dataset/scripts/validate_dataset.py",
        file=sys.stderr,
    )
    sys.exit(1)

try:
    import jsonschema
except ImportError:
    print(
        "Error: jsonschema is required. From the repo root run:\n"
        "  uv sync --group dev\n"
        "  uv run tests/golden_dataset/scripts/validate_dataset.py",
        file=sys.stderr,
    )
    sys.exit(1)


def main():
    dataset_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    schema_dir = os.path.join(dataset_dir, "schema")
    manifest_path = os.path.join(dataset_dir, "manifest.yaml")

    print(f"=== Validating Golden Dataset at: {dataset_dir} ===")
    errors = []

    # 1. Load JSON Schemas
    manifest_schema_path = os.path.join(schema_dir, "manifest.schema.json")
    gold_parent_schema_path = os.path.join(schema_dir, "gold_parent.schema.json")
    gold_ref_schema_path = os.path.join(schema_dir, "gold_references.schema.json")

    for s_path in [manifest_schema_path, gold_parent_schema_path, gold_ref_schema_path]:
        if not os.path.exists(s_path):
            errors.append(f"Missing schema file: {s_path}")

    if errors:
        for err in errors:
            print(f"FAIL: {err}", file=sys.stderr)
        sys.exit(1)

    with open(manifest_schema_path, "r", encoding="utf-8") as f:
        manifest_schema = json.load(f)
    with open(gold_parent_schema_path, "r", encoding="utf-8") as f:
        gold_parent_schema = json.load(f)
    with open(gold_ref_schema_path, "r", encoding="utf-8") as f:
        gold_ref_schema = json.load(f)

    # 2. Parse manifest.yaml and validate schema
    if not os.path.exists(manifest_path):
        print(f"FAIL: Manifest not found at {manifest_path}", file=sys.stderr)
        sys.exit(1)

    try:
        with open(manifest_path, "r", encoding="utf-8") as f:
            manifest_data = yaml.safe_load(f)
    except Exception as e:
        print(f"FAIL: manifest.yaml failed YAML parsing: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        jsonschema.validate(instance=manifest_data, schema=manifest_schema)
        print(" [OK] manifest.yaml schema validation passed")
    except jsonschema.ValidationError as ve:
        errors.append(f"manifest.yaml schema error: {ve.message}")

    # 3. Validate each manifest entry
    sources = manifest_data.get("sources", [])
    print(f" Found {len(sources)} source entries in manifest.yaml")

    for src in sources:
        stem = src.get("source_stem", "unknown")
        rel_fdir = src.get("fixture_dir", "")
        fdir = os.path.join(dataset_dir, rel_fdir)

        if not os.path.isdir(fdir):
            errors.append(f"[{stem}] Fixture directory does not exist: {fdir}")
            continue

        content_path = os.path.join(fdir, "content.md")
        meta_path = os.path.join(fdir, "meta.yaml")
        gold_parent_path = os.path.join(fdir, "gold_parent.yaml")
        gold_ref_path = os.path.join(fdir, "gold_references.yaml")

        # Required files
        if not os.path.exists(content_path):
            errors.append(f"[{stem}] Missing required file: content.md")
        if not os.path.exists(meta_path):
            errors.append(f"[{stem}] Missing required file: meta.yaml")

        # Check content_sha256
        if os.path.exists(content_path) and os.path.exists(meta_path):
            try:
                with open(meta_path, "r", encoding="utf-8") as mf:
                    meta_data = yaml.safe_load(mf)
                with open(content_path, "rb") as cf:
                    actual_sha = hashlib.sha256(cf.read()).hexdigest()
                recorded_sha = meta_data.get("content_sha256")
                if recorded_sha != actual_sha:
                    errors.append(
                        f"[{stem}] SHA256 mismatch in meta.yaml! Recorded={recorded_sha}, Actual={actual_sha}"
                    )
            except Exception as e:
                errors.append(f"[{stem}] Failed reading meta.yaml or content.md: {e}")

        # Check gold_parent.yaml if claimed
        if src.get("gold_parent"):
            if not os.path.exists(gold_parent_path):
                errors.append(f"[{stem}] Manifest indicates gold_parent=true, but gold_parent.yaml is missing")
            else:
                try:
                    with open(gold_parent_path, "r", encoding="utf-8") as gpf:
                        gparent_data = yaml.safe_load(gpf)
                    jsonschema.validate(instance=gparent_data, schema=gold_parent_schema)
                except yaml.YAMLError as ye:
                    errors.append(f"[{stem}] gold_parent.yaml YAML parse error: {ye}")
                except jsonschema.ValidationError as ve:
                    errors.append(f"[{stem}] gold_parent.yaml schema error: {ve.message}")

        # Check gold_references.yaml if claimed
        if src.get("gold_references"):
            if not os.path.exists(gold_ref_path):
                errors.append(f"[{stem}] Manifest indicates gold_references=true, but gold_references.yaml is missing")
            else:
                try:
                    with open(gold_ref_path, "r", encoding="utf-8") as grf:
                        gref_data = yaml.safe_load(grf)
                    jsonschema.validate(instance=gref_data, schema=gold_ref_schema)
                except yaml.YAMLError as ye:
                    errors.append(f"[{stem}] gold_references.yaml YAML parse error: {ye}")
                except jsonschema.ValidationError as ve:
                    errors.append(f"[{stem}] gold_references.yaml schema error: {ve.message}")

    # 4. Check all .yaml / .yml files in dataset parse cleanly
    yaml_files = glob.glob(os.path.join(dataset_dir, "**", "*.yaml"), recursive=True) + \
                 glob.glob(os.path.join(dataset_dir, "**", "*.yml"), recursive=True)

    for yf in yaml_files:
        try:
            with open(yf, "r", encoding="utf-8") as f:
                yaml.safe_load(f)
        except Exception as e:
            rel_path = os.path.relpath(yf, dataset_dir)
            errors.append(f"YAML parse failure in {rel_path}: {e}")

    # Summary
    if errors:
        print(f"\n[FAIL] Validation completed with {len(errors)} error(s):", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        sys.exit(1)
    else:
        print("\n[SUCCESS] Dataset integrity validation passed completely! All schemas, files, and SHA-256 hashes match.")
        sys.exit(0)


if __name__ == "__main__":
    main()
