#!/usr/bin/env python3
"""Download a scientific PDF by DOI, arXiv id, or URL into a target directory.

Usage:
    python download_pdf.py <doi|arxiv|url> <output_dir>

Prints the path of the saved PDF to stdout on success.
Exits non-zero with a message on stderr on failure.
"""

from __future__ import annotations

import re
import sys
import urllib.error
import urllib.request
from pathlib import Path


def classify(target: str) -> tuple[str, str]:
    t = target.strip()
    if t.startswith("http://") or t.startswith("https://"):
        return "url", t
    # arXiv: 1234.5678 or arxiv:1234.5678 or abs/pdf URLs already handled
    m = re.match(r"(?i)^(?:arxiv:)?(\d{4}\.\d{4,5})(v\d+)?$", t)
    if m:
        return "arxiv", m.group(1) + (m.group(2) or "")
    # DOI
    if t.lower().startswith("doi:"):
        t = t[4:].strip()
    if t.startswith("10."):
        return "doi", t
    # bare path-like? treat as URL if looks like domain
    if "." in t and " " not in t:
        return "url", t if t.startswith("http") else "https://" + t
    raise SystemExit(f"unrecognized target (need DOI, arXiv id, or URL): {target}")


def filename_for(kind: str, key: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9._-]+", "_", key)
    return f"{safe}.pdf"


def download(url: str, dest: Path, headers: dict[str, str] | None = None) -> None:
    req_headers = {
        "User-Agent": "sil/0.1 (scientist-in-loop; mailto:sil@localhost)",
        "Accept": "application/pdf,*/*",
    }
    if headers:
        req_headers.update(headers)
    req = urllib.request.Request(url, headers=req_headers)
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = resp.read()
            ctype = resp.headers.get("Content-Type", "")
    except urllib.error.HTTPError as e:
        raise SystemExit(f"HTTP {e.code} fetching {url}: {e.reason}") from e
    except urllib.error.URLError as e:
        raise SystemExit(f"network error fetching {url}: {e.reason}") from e

    if len(data) < 5 or not data.startswith(b"%PDF"):
        # some servers wrap PDFs; still write if content-type says pdf
        if "pdf" not in ctype.lower() and not data.startswith(b"%PDF"):
            preview = data[:200]
            raise SystemExit(
                f"response is not a PDF (content-type={ctype!r}, "
                f"start={preview!r})"
            )
    dest.write_bytes(data)


def resolve_url(kind: str, key: str) -> tuple[str, dict[str, str]]:
    if kind == "url":
        return key, {}
    if kind == "arxiv":
        return f"https://arxiv.org/pdf/{key}.pdf", {}
    if kind == "doi":
        # content negotiation via doi.org
        return f"https://doi.org/{key}", {
            "Accept": "application/pdf",
        }
    raise SystemExit(f"unknown kind {kind}")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(
            "usage: download_pdf.py <doi|arxiv|url> <output_dir>",
            file=sys.stderr,
        )
        return 2
    target, out_dir_s = argv[1], argv[2]
    out_dir = Path(out_dir_s)
    out_dir.mkdir(parents=True, exist_ok=True)

    kind, key = classify(target)
    url, headers = resolve_url(kind, key)
    dest = out_dir / filename_for(kind, key)
    # avoid clobber silently: add numeric suffix
    if dest.exists():
        stem, suf = dest.stem, dest.suffix
        n = 2
        while dest.exists():
            dest = out_dir / f"{stem}_{n}{suf}"
            n += 1

    print(f"downloading {url} → {dest}", file=sys.stderr)
    download(url, dest, headers)
    print(str(dest.resolve()))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
