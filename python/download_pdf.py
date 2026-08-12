#!/usr/bin/env python3
"""Download a scientific PDF by DOI, arXiv id, or URL into a target directory.

Usage:
    python download_pdf.py <doi|arxiv|url> <output_dir>

Prints the path of the saved PDF to stdout on success.
Exits non-zero with a message on stderr on failure.
"""

from __future__ import annotations

import os
import re
import sys
import time
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


def should_retry_exception(exc: Exception) -> bool:
    if isinstance(exc, urllib.error.HTTPError):
        return exc.code == 429 or 500 <= exc.code < 600
    if isinstance(exc, urllib.error.URLError):
        return True
    return False


def download_bytes_with_retry(url: str, headers: dict[str, str] | None = None) -> tuple[bytes, str]:
    req_headers = {
        "User-Agent": "sil/0.1 (scientist-in-loop; mailto:sil@localhost)",
        "Accept": "application/pdf,*/*",
    }
    if headers:
        req_headers.update(headers)

    attempts = 3
    base_delay = 0.25
    last_exc: Exception | None = None

    for attempt in range(1, attempts + 1):
        req = urllib.request.Request(url, headers=req_headers)
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                data = resp.read()
                ctype = resp.headers.get("Content-Type", "")
                return data, ctype
        except Exception as e:
            last_exc = e
            if attempt < attempts and should_retry_exception(e):
                time.sleep(base_delay * (2 ** (attempt - 1)))
                continue
            break

    if isinstance(last_exc, urllib.error.HTTPError):
        raise SystemExit(f"HTTP {last_exc.code} fetching {url}: {last_exc.reason}") from last_exc
    elif isinstance(last_exc, urllib.error.URLError):
        raise SystemExit(f"network error fetching {url}: {last_exc.reason}") from last_exc
    elif last_exc:
        raise SystemExit(f"error fetching {url}: {last_exc}") from last_exc
    else:
        raise SystemExit(f"unknown error fetching {url}")


def download(url: str, dest: Path, headers: dict[str, str] | None = None) -> None:
    data, ctype = download_bytes_with_retry(url, headers)

    if len(data) < 5 or not data.startswith(b"%PDF"):
        if "pdf" not in ctype.lower() and not data.startswith(b"%PDF"):
            preview = data[:200]
            raise SystemExit(
                f"response is not a PDF (content-type={ctype!r}, "
                f"start={preview!r})"
            )

    part = dest.with_suffix(dest.suffix + ".part")
    try:
        part.write_bytes(data)
        os.replace(part, dest)
    except Exception as e:
        if part.exists():
            try:
                part.unlink()
            except OSError:
                pass
        raise SystemExit(f"failed to write destination file {dest}: {e}") from e


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
