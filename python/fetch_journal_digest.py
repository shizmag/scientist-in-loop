#!/usr/bin/env python3
"""
fetch_journal_digest.py — Fetch top peer-reviewed journal publications via Crossref API.
"""

import sys
import json
import urllib.parse
import urllib.request

def fetch_journal_publications(query, limit=10):
    params = {
        "query": query,
        "filter": "type:journal-article",
        "rows": limit,
        "sort": "relevance",
    }
    url = f"https://api.crossref.org/works?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "ScientistInLoop/0.1 (mailto:researcher@scientist-in-loop.org)"}
    )
    
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            data = json.loads(response.read().decode("utf-8"))
            items = data.get("message", {}).get("items", [])
            results = []
            for item in items:
                title_list = item.get("title", [])
                title = title_list[0] if title_list else "Untitled"
                
                authors_raw = item.get("author", [])
                authors_str = ", ".join(
                    f"{a.get('given', '')} {a.get('family', '')}".strip()
                    for a in authors_raw
                ) or "Unknown Authors"
                
                container = item.get("container-title", [])
                journal = container[0] if container else "Journal"
                
                doi = item.get("DOI", "")
                url_str = item.get("URL", f"https://doi.org/{doi}" if doi else "")
                
                published = item.get("published-print") or item.get("published-online") or {}
                date_parts = published.get("date-parts", [[None]])[0]
                year = date_parts[0] if date_parts else None
                
                abstract = item.get("abstract", "No abstract available.")
                # Strip simple XML tags if crossref abstract contains <jats:p>
                if abstract and "<" in abstract and ">" in abstract:
                    import re
                    abstract = re.sub(r"<[^>]+>", "", abstract).strip()
                
                results.append({
                    "doi": doi,
                    "title": title,
                    "authors": authors_str,
                    "journal": journal,
                    "year": year,
                    "abstract_text": abstract,
                    "citation_count": item.get("is-referenced-by-count", 0),
                    "url": url_str,
                    "pdf_url": None,
                })
            return results
    except Exception as e:
        sys.stderr.write(f"Error fetching journal digest: {e}\n")
        return []

def main():
    if len(sys.argv) < 2:
        sys.stderr.write("Usage: fetch_journal_digest.py <query> [limit]\n")
        sys.exit(1)
    
    query = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    
    pubs = fetch_journal_publications(query, limit)
    print(json.dumps(pubs, indent=2))

if __name__ == "__main__":
    main()
