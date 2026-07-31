#!/usr/bin/env python3
"""Parse a PDF with Marker and print markdown/text to stdout.

Usage:
    python parse_with_marker.py <path-to-pdf>

Requires the `marker-pdf` (or `marker`) package when available.
Falls back to a minimal pypdf / raw extraction so `sil parse` can still
operate in constrained environments (tests set SIL_MARKER_STUB instead).

Prints extracted text to stdout. Exits non-zero on failure.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path


def parse_with_marker(pdf: Path) -> str:
    """Try Marker converters in order of known APIs."""
    mode = os.environ.get("SIL_MARKER_MODE", "balance")
    # marker-pdf modern API
    try:
        from marker.converters.pdf import PdfConverter  # type: ignore
        from marker.models import create_model_dict  # type: ignore
        from marker.output import text_from_rendered  # type: ignore

        config = {"mode": mode} if mode else {}
        converter = PdfConverter(artifact_dict=create_model_dict(), config=config)
        rendered = converter(str(pdf))
        text, _, _ = text_from_rendered(rendered)
        return text
    except Exception:
        pass

    try:
        from marker.convert import convert_single_pdf  # type: ignore
        from marker.models import load_all_models  # type: ignore

        models = load_all_models()
        full_text, _, _ = convert_single_pdf(str(pdf), models)
        return full_text
    except Exception:
        pass

    raise RuntimeError("marker not available")


def parse_fallback(pdf: Path) -> str:
    """Best-effort text extraction without Marker."""
    try:
        from pypdf import PdfReader  # type: ignore

        reader = PdfReader(str(pdf))
        parts = []
        for page in reader.pages:
            parts.append(page.extract_text() or "")
        text = "\n\n".join(parts).strip()
        if text:
            return text
    except Exception:
        pass

    # Last resort: return a structured stub noting the file (still useful for FTS tests)
    data = pdf.read_bytes()
    if not data.startswith(b"%PDF"):
        raise SystemExit(f"not a PDF: {pdf}")
    return (
        f"# {pdf.name}\n\n"
        f"(Marker unavailable; raw PDF of {len(data)} bytes accepted without text extraction.)\n"
    )


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: parse_with_marker.py <path-to-pdf>", file=sys.stderr)
        return 2
    pdf = Path(argv[1])
    if not pdf.is_file():
        print(f"file not found: {pdf}", file=sys.stderr)
        return 1
    try:
        text = parse_with_marker(pdf)
    except Exception as e:
        print(f"marker failed ({e}); using fallback", file=sys.stderr)
        try:
            text = parse_fallback(pdf)
        except SystemExit:
            raise
        except Exception as e2:
            print(f"parse failed: {e2}", file=sys.stderr)
            return 1
    sys.stdout.write(text)
    if text and not text.endswith("\n"):
        sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
