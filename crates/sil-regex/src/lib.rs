//! Centralized regular expressions and text pattern matchers for scientist-in-loop.

#![deny(missing_docs)]

use regex::Regex;
use std::sync::LazyLock;

static DOI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b10\.\d{4,9}/[-._;()/:A-Za-z0-9]+\b").unwrap());

static ARXIV_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:arxiv:\s*)?(\d{4}\.\d{4,5}(?:v\d+)?|[a-z\-]+(?:\.[a-z\-]+)?/\d{7}(?:v\d+)?)\b",
    )
    .unwrap()
});

static OPENREVIEW_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)https?://(?:www\.)?openreview\.net/(?:forum|pdf)\?id=([A-Za-z0-9_-]{10,12})")
        .unwrap()
});

static OPENREVIEW_PREFIX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bopenreview:\s*([A-Za-z0-9_-]{10,12})\b").unwrap()
});

static OPENREVIEW_RAW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Za-z0-9_-]{10,12})\b").unwrap()
});

static YEAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(1[89]\d{2}|20[0-2]\d|2030)\b").unwrap());

static QUOTED_TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"["“]([^"”\r\n]{2,})[”"]"#).unwrap());

static REF_HEADING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*#*\s*(?:\d+\.?)?\s*(?:\*\*|__)?\s*(references|bibliography|literature cited|works cited|references\s*(?:and|&)\s*notes|online content|selected references|additional references)(?:\*\*|__)?\b").unwrap()
});

static NON_REF_HEADING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)^\s*#*\s*(?:\d+\.?)?\s*(?:[A-Z0-9]\.?\s+)?(?:\*\*|__)?\s*",
        r"(appendix|proofs?|pseudocode|algorithm|listings?|author contributions|acknowledgements|acknowledgments|figures|tables|supplementary|supplemental|ethics statement|declarations|competing interests|conflict of interest|about the authors|biography|author biographies)(?:\*\*|__)?\b"
    )).unwrap()
});

static REF_ENTRY_START_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"^\s*(?:[\-*•]\s+)?(?:<span[^>]*>.*?</span>\s*)?"#,
        r#"(?:"#,
        // 1. Bracketed or numbered markers: [1], (1), 1., 1), [Vaswani 2017], (Vaswani 2017)
        r#"\[\d+\]|\(\d+\)|\d+[\.\)]|\([^\)]*\d{4}\)|\[[^\]]*\d{4}\]"#,
        r#"|"#,
        // 2. Surname, Initial / Surname, Firstname: "Vaswani, A.", "Der Kiureghian, A."
        r#"[A-Z][a-zA-Za-z\-']+[,\;\:]\s+[A-Z]"#,
        r#"|"#,
        // 3. Firstname [Middle] Surname, / . : "Sourya Basu,", "Katherine Tian,", "Nelson F Liu,", "David JC MacKay."
        r#"[A-Z][a-zA-Za-z\-']+(?:\s+[A-Z]{1,3}\.?|\s+[A-Z][a-zA-Za-z\-']+)+\s*[,\.]\s*[*_"'“]?[A-Z]"#,
        r#"|"#,
        // 4. Initials Surname : "J. D. Hunter.", "J. Platt."
        r#"[A-Z]\.(?:\s+[A-Z]\.)*\s+[A-Z][a-zA-Za-z\-']+[,\;\.]"#,
        r#")"#,
        r#"|"#,
        // 5. Prefixed "Name and Name" / "Name et al" / "Org maintainers" entries
        r#"^\s*(?:[\-*•]\s+(?:<span[^>]*>.*?</span>\s*)?|<span[^>]*>.*?</span>\s*)"#,
        r#"(?:"#,
        r#"[A-Z][a-zA-Za-z\-']+(?:\s+[A-Z]{1,3}\.?|\s+[A-Z][a-zA-Za-z\-']+)+\s+(?:and|&)\s+[A-Z][a-zA-Za-z\-']+"#,
        r#"|"#,
        r#"[A-Z][a-zA-Za-z\-']+(?:\s+[A-Z][a-zA-Za-z\-']+)*\s+et\s+al"#,
        r#"|"#,
        r#"[A-Z][a-zA-Za-z\-']+(?:\s+[a-zA-Za-z\-']+)*\s+(?:maintainers|contributors|authors|team|group|collaboration|consortium|committee|editors?)\b"#,
        r#")"#
    )).unwrap()
});

static LATEX_METADATA_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*%\s*metadata:\s*(.+)$").unwrap());

static HTML_SPAN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<span[^>]*>|</span>").unwrap());

static A_TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<a[^>]*>|</a>").unwrap());

static MD_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap());

static AUTHOR_FOOTNOTE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<sup>.*?</sup>|\[[a-z0-9*\\†‡,∗ ]+\]|[*†‡§¶#\\∗]+").unwrap());

static INLINE_BULLET_SEP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\.|\b)\s+[\-*•]\s+([A-Z])").unwrap());

static AND_AUTHOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:and|&)\s+[A-Z][a-zA-Za-z\-']+(?:\s+[A-Z][a-zA-Za-z\-']+)?$").unwrap()
});

static SPLIT_MD_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([A-Za-z]+)\[([A-Za-z]+)\]\([^)]+\)").unwrap());

static ORCID_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[ID\]\([^)]+\)").unwrap());

static EMAIL_BRACKETS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{[^}]*\}@[^\s,]+").unwrap());

static EMAIL_ADDR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:email:\s*)?[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b").unwrap()
});

static IEEE_BADGE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\*?\s*(?:Senior|Student|Fellow)?\s*(?:Member)?,\s*IEEE\*?").unwrap()
});

static AND_SPLIT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s+(?:and|&)\s+").unwrap());

/// Unprefixed dual-author bibliography starts: "Amos Azaria and Tom Mitchell. 2023. …"
/// Used when Marker spans were already stripped (splitter cleans lines first).
static NAME_AND_NAME_START_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"^\s*"#,
        r#"[A-Z][a-zA-Za-z\-']+(?:\s+[A-Z]{1,3}\.?|\s+[A-Z][a-zA-Za-z\-']+)+\s+"#,
        r#"(?:and|&)\s+"#,
        r#"[A-Z][a-zA-Za-z\-']+(?:\s+[A-Z]{1,3}\.?|\s+[A-Z][a-zA-Za-z\-']+)*"#,
        r#"\s*[,\.]"#,
    ))
    .unwrap()
});

/// Expand inline bullet separators (e.g. `. - Author`) into newlines.
pub fn expand_inline_bullet_references(text: &str) -> String {
    INLINE_BULLET_SEP_REGEX
        .replace_all(text, "$1\n- $2")
        .to_string()
}

/// Check if a text candidate matches author list patterns (e.g. "Author 1, Author 2, and Author 3").
pub fn is_author_list(candidate: &str) -> bool {
    let t = candidate.trim();
    if t.is_empty() {
        return false;
    }

    if AND_AUTHOR_REGEX.is_match(t) {
        return true;
    }

    let comma_parts: Vec<&str> = t.split(',').map(|s| s.trim()).collect();
    if comma_parts.len() >= 2 {
        let mut name_like_parts = 0;
        for part in &comma_parts {
            let words: Vec<&str> = part.split_whitespace().collect();
            if !words.is_empty() && words.len() <= 4 {
                let all_capitalized = words.iter().all(|w| {
                    w.chars().next().is_some_and(|c| c.is_uppercase())
                        || *w == "and"
                        || *w == "&"
                        || *w == "de"
                        || *w == "van"
                        || *w == "von"
                        || *w == "der"
                });
                if all_capitalized {
                    name_like_parts += 1;
                }
            }
        }
        if name_like_parts >= comma_parts.len() - 1 && name_like_parts >= 2 {
            return true;
        }
    }

    false
}

/// Strip HTML `<span...>` and `</span>` tags from text.
pub fn strip_html_spans(text: &str) -> String {
    HTML_SPAN_REGEX.replace_all(text, "").to_string()
}

/// Strip markdown links like `[Name](#page-1-0)` to `Name`.
///
/// Uses a simple non-nested regex; prefer [`map_markdown_links`] when URLs may
/// contain parentheses (e.g. Elsevier `refhub` paths with `(26)`).
pub fn strip_markdown_links(text: &str) -> String {
    MD_LINK_REGEX.replace_all(text, "$1").to_string()
}

/// Map markdown links `[text](url)` with **balanced** parentheses in `url`.
///
/// Standard regex `[^)]+` stops at the first `)`, which corrupts Elsevier refhub
/// links such as `[hallucinations](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb2)`
/// into `hallucinations00077-7/sb2)`.
pub fn map_markdown_links(text: &str, mut map: impl FnMut(&str, &str) -> String) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'['
            && let Some(close_bracket) = text[i + 1..].find(']').map(|o| i + 1 + o)
            && close_bracket + 1 < bytes.len()
            && bytes[close_bracket + 1] == b'('
        {
            let mut depth = 1usize;
            let mut k = close_bracket + 2;
            while k < bytes.len() && depth > 0 {
                match bytes[k] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                k += 1;
            }
            if depth == 0 {
                let link_text = &text[i + 1..close_bracket];
                let url = &text[close_bracket + 2..k - 1];
                out.push_str(&map(link_text, url));
                i = k;
                continue;
            }
        }
        out.push(text[i..].chars().next().unwrap());
        i += text[i..].chars().next().unwrap().len_utf8();
    }
    out
}

/// Strip author footnote markers like `<sup>...</sup>`, `[\*1]`, `[a]`, `\*`, `†`, etc.
pub fn strip_author_footnote_markers(text: &str) -> String {
    AUTHOR_FOOTNOTE_REGEX.replace_all(text, "").to_string()
}

/// Clean reference text: strips HTML span/a tags, markdown links (except DOI/arXiv), normalizes spaces, trims list prefixes.
pub fn clean_reference_text(text: &str) -> String {
    let mut cleaned = HTML_SPAN_REGEX.replace_all(text, "").to_string();
    cleaned = A_TAG_REGEX.replace_all(&cleaned, "").to_string();

    // Balanced-paren aware: preserve DOI/arXiv markdown links, drop ACL noise, keep link text.
    cleaned = map_markdown_links(&cleaned, |text_content, url| {
        let url_lower = url.to_lowercase();
        if url.contains("10.") || url_lower.contains("arxiv") || extract_arxiv_id(url).is_some() {
            format!("[{text_content}]({url})")
        } else if text_content.contains("aclanthology.org")
            || url_lower.contains("aclanthology.org")
        {
            String::new()
        } else if url_lower.contains("refhub.elsevier.com") || url_lower.contains("refhub") {
            // Elsevier Marker dumps often link single words mid-title; keep the word only.
            text_content
                .trim_end_matches([',', '.', ':', ';'])
                .to_string()
        } else {
            text_content.to_string()
        }
    });

    // Safety net for already-broken or partially-stripped Elsevier fragments.
    static REFHUB_RESIDUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(?:https?://)?(?:refhub\.elsevier\.com/)?S?\d{4}-\d{3,5}X?\([^)]*\)?\d*/?sb\d+\)?|\b\d{4,5}-\d+/sb\d+\)?").unwrap()
    });
    cleaned = REFHUB_RESIDUE_REGEX.replace_all(&cleaned, "").to_string();

    static ACL_URL_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)https?://(?:www\.)?aclanthology\.org/\S*").unwrap());
    cleaned = ACL_URL_REGEX.replace_all(&cleaned, "").to_string();

    static FIGURE_CAPTION_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\s*\([a-d]\)\s*(?:Histogram|Figure|OoD|Detection|Table)\b.*$").unwrap()
    });
    cleaned = FIGURE_CAPTION_SUFFIX.replace(&cleaned, "").to_string();

    static MULTI_SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s{2,}").unwrap());
    cleaned = MULTI_SPACE_REGEX.replace_all(&cleaned, " ").to_string();
    // Rejoin words split by stripped mid-title links: "graph– large" / "model :".
    cleaned = cleaned.replace("– ", "–").replace("— ", "—");
    static SPACE_BEFORE_PUNCT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\s+([,.;:!?])").unwrap());
    cleaned = SPACE_BEFORE_PUNCT.replace_all(&cleaned, "$1").to_string();

    let mut trimmed = cleaned.trim();
    if let Some(idx) = trimmed.find(']') {
        if trimmed.starts_with('[') || trimmed.starts_with("- [") {
            trimmed = &trimmed[idx + 1..];
        }
    } else if let Some(pos) = trimmed.find(". ") {
        let prefix = &trimmed[..pos];
        let num_prefix = prefix.trim_start_matches('-');
        if num_prefix.trim().chars().all(|c| c.is_ascii_digit()) {
            trimmed = &trimmed[pos + 2..];
        }
    }
    trimmed.trim_start_matches('-').trim().to_string()
}

/// Normalize a markdown heading / frontmatter label for section matching.
///
/// Strips HTML spans, markdown `#` markers, list bullets, and bold/italic
/// markers so that `#### **Abstract**` and `## **1 Introduction**` compare as
/// `abstract` and `1 introduction`.
pub fn normalize_heading_text(line: &str) -> String {
    let mut s = strip_html_spans(line);
    s = s.trim().to_string();
    // Markdown heading markers
    while s.starts_with('#') {
        s = s[1..].trim_start().to_string();
    }
    // Leading list bullets: "- Date:", "* Code:"
    if let Some(rest) = s
        .strip_prefix("- ")
        .or_else(|| s.strip_prefix("* "))
        .or_else(|| s.strip_prefix("• "))
    {
        s = rest.trim().to_string();
    }
    // Bold/italic wrappers and leftover emphasis markers
    s = s.replace("**", "").replace("__", "");
    s = s
        .trim_matches(|c: char| c == '*' || c == '_' || c == '`' || c == '"' || c == '\'')
        .trim()
        .to_string();
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// True when a line ends parent frontmatter / byline scanning (abstract,
/// introduction, keywords, date/code meta bullets, etc.).
pub fn is_frontmatter_section_stop(line: &str) -> bool {
    let n = normalize_heading_text(line);
    if n.is_empty() {
        return false;
    }
    // Abstract / keywords: allow `*Abstract*—…` and spaced `A B S T R A C T`.
    // Do NOT use bare `data ` / `code ` prefixes — those false-stop Elsevier
    // journal titles like "Data & Knowledge Engineering".
    if n == "abstract"
        || n == "a b s t r a c t"
        || n.starts_with("abstract")
        || n == "contents"
        || n == "a r t i c l e i n f o"
        || n.starts_with("article info")
        || n.starts_with("keywords")
        || n.starts_with("index terms")
        || n.starts_with("1 introduction")
        || n.starts_with("1. introduction")
        || n.starts_with("i. introduction")
        || n.starts_with("i introduction")
        || n == "introduction"
        || n.starts_with("date:")
        || n.starts_with("correspondence:")
        || n.starts_with("code:")
        || n.starts_with("data:")
    {
        return true;
    }
    // Numbered section starts that never belong in the byline region.
    static NUMBERED_BODY_SECTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\d+(\.\d+)*\s+(introduction|related work|background|preliminar|method|experiments?)\b",
        )
        .unwrap()
    });
    NUMBERED_BODY_SECTION.is_match(&n)
}

/// Check if line contains affiliation or noise keywords
pub fn is_affiliation_or_noise_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    let keywords = [
        "university",
        "department",
        "school of",
        "institute of",
        "faculty of",
        "laboratory",
        "lab",
        "inc.",
        "corplab",
        "address",
        "a r t i c l e i n f o",
        "contents lists",
        "journal homepage:",
        "received",
        "accepted",
        "available online",
        "@",
        "equal contribution",
        "author to whom",
        "correspondence",
        "corresponding author",
        "abstract",
        "a b s t r a c t",
        "introduction",
        "keywords",
        "index terms",
        "date:",
        "code:",
        "data:",
        "https://github",
        "github.com",
        "huggingface.co",
        "https://huggingface",
        "reconstructing",
        "our contributions",
        "contributions",
        "the work was done",
        "lt;",
        "gt;",
        "preliminaries",
        "methodology",
        "problem formulation",
        "background",
        "related work",
        "table of contents",
        "contents",
    ];
    keywords.iter().any(|&k| lower.contains(k))
}

/// Check if a candidate line is publisher chrome or a known journal title header.
pub fn is_journal_or_publisher_title(line: &str) -> bool {
    let clean = strip_html_spans(line).trim().to_string();
    let raw = clean
        .trim_start_matches('#')
        .trim()
        .trim_matches('*')
        .trim_matches('_')
        .trim();
    let lower = raw.to_lowercase();

    if lower.is_empty() {
        return true;
    }

    let bad_exact = [
        "sciencedirect",
        "knowledge-based systems",
        "data & knowledge engineering",
        "intelligent systems with applications",
        "abstract",
        "a b s t r a c t",
        "contents",
        "paper under double-blind review",
        "article info",
        "a r t i c l e i n f o",
    ];
    if bad_exact.contains(&lower.as_str()) {
        return true;
    }

    let bad_prefixes = [
        "contents lists available at",
        "journal homepage",
        "1 introduction",
        "1. introduction",
        "i. introduction",
        "parsed from",
    ];
    if bad_prefixes.iter().any(|&p| lower.starts_with(p)) {
        return true;
    }

    if lower.contains("elsevier") {
        return true;
    }

    false
}

/// Clean an author line from byline section: strips links, ORCIDs, emails, math footnotes, IEEE badges, noise.
pub fn clean_author_byline_line(line: &str) -> String {
    let mut s = line.trim().to_string();

    if s.starts_with('#') {
        s = s.trim_start_matches('#').trim().to_string();
    }

    // Handle split markdown links e.g. She[n](https://...) -> Shen
    s = SPLIT_MD_LINK.replace_all(&s, "$1$2").to_string();

    // Strip ORCID link icon e.g. [ID](https://orcid.org/...)
    s = ORCID_LINK_REGEX.replace_all(&s, "").to_string();

    // General markdown link stripping [Name](#link) -> Name
    s = strip_markdown_links(&s);

    // Strip email blocks e.g. {user1, user2}@domain.com or user1@domain.com
    s = EMAIL_BRACKETS_REGEX.replace_all(&s, "").to_string();
    s = EMAIL_ADDR_REGEX.replace_all(&s, "").to_string();

    // Strip attached email usernames e.g. Ernesto Quevedo1@Baylor.edu -> Ernesto Quevedo
    static EMAIL_USERNAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b").unwrap()
    });
    s = EMAIL_USERNAME_REGEX.replace_all(&s, "").to_string();

    // Handle affiliation lines and keywords
    if let Some(idx) = find_affiliation_keyword_idx(&s) {
        let before = s[..idx].trim().to_string();
        // Pure affiliation lines like "Gaoling School of …" leave an orphan campus
        // token; keep mononyms ("Harshavardhan Independent Researcher").
        if before.is_empty() {
            return String::new();
        }
        let s_lower = s.to_lowercase();
        let looks_like_campus_orphan = before.split_whitespace().count() == 1
            && (s_lower.contains("school")
                || s_lower.contains("university")
                || s_lower.contains("institute")
                || s_lower.contains("department")
                || s_lower.contains("laboratory")
                || s_lower.contains(" lab"));
        if looks_like_campus_orphan {
            return String::new();
        }
        s = before;
    } else if is_affiliation_or_noise_line(&s) {
        return String::new();
    }

    // Strip TeX math footnote markers e.g. $^{1*\dagger}$, $^1$, $^{2\ddagger}$.
    // Keep alternatives tight — a greedy `$^…$` spanning multiple math groups
    // would erase intervening author names (Jing Liu between `$^2$` and `$^{2}$`).
    static TEX_MATH_NOISE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!(
            r"\$\^\{[^}]*\}\$",          // $^{1*\dagger}$
            r"|\$\^[A-Za-z0-9*†‡\\]+\$", // $^2$, $^1*$
            r"|\$[^$]{1,24}\$",          // other short inline math
            r"|\^\{[^}]*\}",             // ^{1} without dollars
            r"|\^\[[^\]]*\]",            // ^[1]
        ))
        .unwrap()
    });
    s = TEX_MATH_NOISE_REGEX.replace_all(&s, "").to_string();

    // Strip HTML <sup>...</sup>
    s = strip_author_footnote_markers(&s);

    // Strip footnote/superscript characters
    static NOISE_CHARS: &[char] = &[
        '*', '⋈', '†', '‡', '§', '¶', '♯', '♠', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', 'ⁿ', '՞', 'ã',
        'ゥ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    ];
    s = s.replace(NOISE_CHARS, "");

    // Strip IEEE badges
    s = IEEE_BADGE_REGEX.replace_all(&s, "").to_string();

    // Marker often separates authors with multi-spaces (or spaces left after
    // stripping `$^{…}$`). Promote those gaps to commas before collapse so
    // "Yuhao Wang  Ruiyang Ren  Wayne Xin Zhao" splits correctly.
    static MULTI_SPACE_SEP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]{2,}").unwrap());
    s = MULTI_SPACE_SEP.replace_all(&s, ", ").to_string();

    s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.trim_matches(|c: char| c == ',' || c == ';' || c == '-' || c.is_whitespace())
        .to_string()
}

fn find_affiliation_keyword_idx(text: &str) -> Option<usize> {
    let lower = text.to_lowercase();
    let keywords = [
        "independent researcher",
        "departamento de",
        "departamento",
        "department of",
        "department",
        "school of",
        "school",
        "universidad de",
        "universidad",
        "university of",
        "university",
        "faculty of",
        "faculty",
        "college of",
        "college",
        "institute of",
        "institute",
        "laboratory",
        "lab",
        "corp lab",
        "corplab",
        "baidu inc",
        "adobe research",
        "amazon",
        "meta",
        "snap inc",
        "home depot",
    ];
    keywords.iter().filter_map(|kw| lower.find(kw)).min()
}

/// Score how much `middle` looks like a middle name/initial given the first name.
/// Higher is better; 0 means "not a middle name".
fn middle_name_score(first: &str, middle: &str) -> i32 {
    let t = middle.trim_matches(|c: char| c == '.' || c == ',');
    if t.is_empty() || !t.chars().next().is_some_and(|c| c.is_uppercase()) {
        return 0;
    }
    // Initials: "A", "A."
    if middle.ends_with('.') || t.len() == 1 {
        if t.chars().all(|c| c.is_ascii_alphabetic() || c == '.') {
            return 100;
        }
        return 0;
    }
    if !t.chars().all(|c| c.is_ascii_alphabetic()) {
        return 0;
    }
    // Never treat 1–2 letter tokens as middles ("Lu", "Wu" surnames).
    if t.len() < 3 || t.len() > 4 {
        return 0;
    }
    // Prefer middles after longer given names ("Wayne Xin") over short ones.
    let bonus = if first.len() >= 5 { 20 } else { 0 };
    if t.len() == 3 {
        50 + bonus
    } else {
        // len == 4 ("Paul"): weaker than 3-letter middles.
        30 + bonus
    }
}

/// Split author line into individual candidate author names.
pub fn split_author_names(line: &str) -> Vec<String> {
    let mut res = Vec::new();
    if line.contains(',') || line.contains(';') {
        let parts: Vec<&str> = line.split(&[';', ','][..]).collect();
        for part in parts {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            let and_split: Vec<&str> = AND_SPLIT_REGEX.split(trimmed).collect();
            for item in and_split {
                let clean_item = item
                    .trim()
                    .trim_matches(|c: char| c == ',' || c == ';' || c.is_whitespace())
                    .to_string();
                if !clean_item.is_empty() {
                    res.push(clean_item);
                }
            }
        }
    } else {
        // No commas: prefer First Last pairs. When the word count is odd, place
        // exactly one First Middle Last triple at the best-scoring even index
        // (e.g. "Wayne Xin Zhao" amid Chinese First Last pairs).
        let words: Vec<&str> = line.split_whitespace().collect();
        let n = words.len();
        let mut triple_at: Option<usize> = None;
        if n >= 3 && !n.is_multiple_of(2) {
            let mut best_i = None;
            let mut best_score = 0;
            let mut i = 0;
            while i + 2 < n {
                let rem_after = n - (i + 3);
                if rem_after.is_multiple_of(2) {
                    let score = middle_name_score(words[i], words[i + 1]);
                    // Prefer higher score; break ties toward the right.
                    if score > 0 && score >= best_score {
                        best_score = score;
                        best_i = Some(i);
                    }
                }
                i += 2;
            }
            triple_at = best_i;
        }

        let mut idx = 0;
        while idx < n {
            let w1 = words[idx];
            if triple_at == Some(idx) && idx + 2 < n {
                res.push(format!(
                    "{} {} {}",
                    words[idx],
                    words[idx + 1],
                    words[idx + 2]
                ));
                idx += 3;
                continue;
            }
            if idx + 1 < n {
                let w2 = words[idx + 1];
                let is_w1_cap = w1.chars().next().is_some_and(|c| c.is_uppercase());
                let is_w2_cap = w2.chars().next().is_some_and(|c| c.is_uppercase());
                if is_w1_cap && is_w2_cap {
                    res.push(format!("{w1} {w2}"));
                    idx += 2;
                    continue;
                }
            }
            if w1.chars().next().is_some_and(|c| c.is_uppercase()) {
                res.push(w1.to_string());
            }
            idx += 1;
        }
    }
    res
}

/// Extract publication year from document header line (does not scan body text).
pub fn extract_header_year(line: &str) -> Option<i32> {
    let lower = line.to_lowercase();
    if lower.contains("published:")
        || lower.contains("published online:")
        || lower.contains("date:")
        || lower.contains("received:")
        || lower.contains("accepted:")
        || lower.contains("available online")
        || lower.contains("©")
        || lower.contains("copyright")
        || lower.contains("may 20")
        || lower.contains("sep 20")
        || lower.contains("jan 20")
        || lower.contains("feb 20")
        || lower.contains("mar 20")
        || lower.contains("apr 20")
        || lower.contains("jun 20")
        || lower.contains("jul 20")
        || lower.contains("aug 20")
        || lower.contains("oct 20")
        || lower.contains("nov 20")
        || lower.contains("dec 20")
        || lower.contains("10.1016/")
        || lower.contains("10.1038/")
    {
        return extract_year(line);
    }
    None
}

/// Check if line is an author biography or prose line (not a citation).
pub fn is_biography_or_prose_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    let bio_keywords = [
        "received his b.",
        "received her b.",
        "received his m.",
        "received her m.",
        "received his ph.d",
        "received her ph.d",
        "received his bscs",
        "received her bscs",
        "received his mscs",
        "received her mscs",
        "is currently a ph.d",
        "is currently a professor",
        "senior researcher",
        "research group leader",
        "his research interests",
        "her research interests",
        "full professor",
        "assistant professor",
        "associate professor",
        "research assistant",
        "member of ieee",
        "member of acm",
        "member of dbsj",
        "board member of",
        "this decomposition shows",
        "implications for large models",
        "the epistemic uncertainty",
    ];
    bio_keywords.iter().any(|&k| lower.contains(k))
}

/// Check if a line is a double-blind margin line number line (e.g. "**558 559 560**" or "**564**").
pub fn is_margin_line_number(line: &str) -> bool {
    let s = strip_html_spans(line);
    let clean = s.trim();
    if clean.is_empty() {
        return false;
    }
    let t = clean.trim_start_matches("**").trim_end_matches("**").trim();
    !t.is_empty()
        && t.split_whitespace()
            .all(|w| w.chars().all(|c| c.is_ascii_digit()))
}

/// Check if text has strong publication / citation markers (doi, arxiv, in:, pp., vol., journal/conf keywords).
pub fn has_strong_citation_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("doi:")
        || lower.contains("arxiv:")
        || lower.contains("https://")
        || lower.contains("http://")
        || lower.contains("in:")
        || lower.contains("pp.")
        || lower.contains("vol.")
        || lower.contains("no.")
        || lower.contains("isbn")
        || lower.contains("proceedings")
        || lower.contains("journal")
        || lower.contains("conference")
        || lower.contains("transactions")
        || lower.contains("symposium")
        || lower.contains("workshop")
        || lower.contains("press")
        || lower.contains("publisher")
}

/// Extract a DOI (Digital Object Identifier) from text.
///
/// Matches pattern `10.XXXX/...` and strips trailing punctuation (`.`, `,`, `;`, `)`, `]`).
pub fn extract_doi(text: &str) -> Option<String> {
    DOI_REGEX.find(text).map(|m| {
        m.as_str()
            .trim_end_matches(&['.', ',', ';', ')', ']'][..])
            .to_string()
    })
}

static GENERIC_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bhttps?://[^\s<>]+|\bURL\s*<([^>]+)>").unwrap());

/// Extract an arXiv identifier from text (e.g. `1706.03762` or `arxiv:1706.03762v1`).
pub fn extract_arxiv_id(text: &str) -> Option<String> {
    ARXIV_REGEX.find(text).map(|m| {
        m.as_str()
            .trim_end_matches(&['.', ',', ';', ')', ']'][..])
            .to_string()
    })
}

/// Extract an OpenReview note identifier from text.
///
/// Detects OpenReview URLs (`https://openreview.net/forum?id=XXX`, `https://openreview.net/pdf?id=XXX`),
/// `openreview:XXX`, or raw OpenReview note IDs (10-12 alphanumeric characters like `uccHPGDlao`).
pub fn extract_openreview_id(text: &str) -> Option<String> {
    if let Some(caps) = OPENREVIEW_URL_REGEX.captures(text)
        && let Some(m) = caps.get(1)
    {
        return Some(m.as_str().to_string());
    }
    if let Some(caps) = OPENREVIEW_PREFIX_REGEX.captures(text)
        && let Some(m) = caps.get(1)
    {
        return Some(m.as_str().to_string());
    }
    let text_lower = text.to_lowercase();
    let trimmed = text.trim().trim_end_matches(&['.', ',', ';', ')', ']', '>'][..]);
    for caps in OPENREVIEW_RAW_REGEX.captures_iter(trimmed) {
        if let Some(m) = caps.get(1) {
            let val = m.as_str();
            if val.eq_ignore_ascii_case("openreview") {
                continue;
            }
            if trimmed == val
                || text_lower.contains("openreview")
                || text_lower.contains("forum")
                || text_lower.contains("note")
                || text_lower.contains("id")
                || val.chars().any(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
            {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Extract any URL (e.g. `https://github.com/facebookresearch/xformers` or `https://arxiv.org/abs/2501.12948`) from text.
pub fn extract_url(text: &str) -> Option<String> {
    if let Some(caps) = GENERIC_URL_REGEX.captures(text) {
        if let Some(group1) = caps.get(1) {
            return Some(
                group1
                    .as_str()
                    .trim_matches(&[' ', '>', '<', '.', ','][..])
                    .to_string(),
            );
        }
        if let Some(m) = caps.get(0) {
            return Some(
                m.as_str()
                    .trim_matches(&[' ', '>', '<', '.', ',', ')', ']'][..])
                    .to_string(),
            );
        }
    }
    None
}

/// Extract a 4-digit publication year between 1800 and 2030 from text.
pub fn extract_year(text: &str) -> Option<i32> {
    for mat in YEAR_REGEX.find_iter(text) {
        if let Ok(y) = mat.as_str().parse::<i32>()
            && (1800..=2030).contains(&y)
        {
            return Some(y);
        }
    }
    None
}

/// Extract a quoted title from text (matching straight double quotes or curly quotes).
pub fn extract_quoted_title(text: &str) -> Option<String> {
    QUOTED_TITLE_REGEX
        .captures(text)
        .map(|caps| caps.get(1).unwrap().as_str().trim().to_string())
}

/// Check if a line is a reference section heading (e.g., `# References`, `Bibliography`, `8. References`).
pub fn is_reference_heading(line: &str) -> bool {
    REF_HEADING_REGEX.is_match(line)
}

/// Check if a line is a non-reference heading signaling the end of reference section
/// (e.g., `Appendix`, `Author contributions`, `Acknowledgements`, `Figures`, `Tables`, `Supplementary`).
pub fn is_non_ref_heading(line: &str) -> bool {
    NON_REF_HEADING_REGEX.is_match(line)
}

/// Check if a line starts a reference list entry (e.g., `[1]`, `1.`, `[Vaswani 2017]`, `(1)`).
pub fn is_reference_entry_start(line: &str) -> bool {
    let s = strip_html_spans(line);
    let trimmed = s
        .trim()
        .trim_start_matches('-')
        .trim_start_matches('*')
        .trim_start_matches('•')
        .trim();

    let first_word = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphabetic());

    let non_author_words = [
        "Language",
        "Model",
        "Models",
        "System",
        "Systems",
        "Dataset",
        "Datasets",
        "Survey",
        "Review",
        "Overview",
        "Report",
        "Technical",
        "Evaluation",
        "Benchmark",
        "Deep",
        "Neural",
        "Learning",
        "Artificial",
        "Computer",
        "Vision",
        "Natural",
        "Information",
        "Figure",
        "Table",
        "Section",
        "Appendix",
        "Algorithm",
        "Listing",
    ];
    if non_author_words.contains(&first_word) {
        return false;
    }

    if REF_ENTRY_START_REGEX.is_match(line) || REF_ENTRY_START_REGEX.is_match(&s) {
        return true;
    }

    // After the splitter strips `<span>` tags, dual-author lines lose their only
    // structural prefix. Treat "First Last and First Last. YEAR" as entry starts
    // when a publication year is present (e.g. Amos Azaria and Tom Mitchell. 2023.).
    if extract_year(trimmed).is_some() && NAME_AND_NAME_START_REGEX.is_match(trimmed) {
        return true;
    }

    false
}

/// Extract LaTeX `% metadata: ...` comment content from a line.
pub fn extract_latex_metadata_comment(line: &str) -> Option<String> {
    LATEX_METADATA_REGEX
        .captures(line)
        .map(|caps| caps.get(1).unwrap().as_str().trim().to_string())
}

/// Extract journal / conference / venue name from a reference text line.
pub fn extract_reference_venue(text: &str) -> Option<String> {
    let clean = strip_html_spans(text);

    // Explicit venue keywords & patterns
    let patterns = [
        "Nature",
        "Science",
        "Cell",
        "PNAS",
        "Nucleic Acids Research",
        "Knowledge-Based Systems",
        "Data & Knowledge Engineering",
        "ACM Computing Surveys",
        "Findings of the Association for Computational Linguistics",
        "Findings of ACL",
        "Association for Computational Linguistics",
        "NeurIPS",
        "NIPS",
        "Advances in Neural Information Processing Systems",
        "ICML",
        "International Conference on Machine Learning",
        "ICLR",
        "International Conference on Learning Representations",
        "CVPR",
        "IEEE/CVF Conference on Computer Vision and Pattern Recognition",
        "ICCV",
        "ECCV",
        "EMNLP",
        "NAACL",
        "ACL",
        "AAAI",
        "IJCAI",
        "KDD",
        "SIGIR",
        "IEEE Transactions",
        "ACM Transactions",
        "CoRR",
        "arXiv",
    ];

    for pat in patterns {
        if clean.contains(pat) {
            return Some(pat.to_string());
        }
    }

    // Generic match for "Proceedings of ..." or "Journal of ..."
    if let Some(pos) = clean.find("Proceedings of ") {
        let rest = &clean[pos..];
        let end = rest
            .find('.')
            .or_else(|| rest.find(','))
            .unwrap_or(rest.len());
        let candidate = rest[..end].trim();
        if candidate.len() > 10 && candidate.len() < 120 {
            return Some(candidate.to_string());
        }
    }

    if let Some(pos) = clean.find("Journal of ") {
        let rest = &clean[pos..];
        let end = rest
            .find('.')
            .or_else(|| rest.find(','))
            .unwrap_or(rest.len());
        let candidate = rest[..end].trim();
        if candidate.len() > 10 && candidate.len() < 120 {
            return Some(candidate.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_author_byline_line_single_author() {
        let line = "Harshavardhan Independent Researcher harsh@link.cuhk.edu.hk";
        let cleaned = clean_author_byline_line(line);
        let names = split_author_names(&cleaned);
        assert_eq!(cleaned, "Harshavardhan");
        assert_eq!(names, vec!["Harshavardhan"]);
    }

    #[test]
    fn test_split_author_names_space_separated_pairs_and_middle() {
        let paired = split_author_names("Wensheng Lu Keyu Chen Ruizhi Qiao Xing Sun");
        assert_eq!(
            paired,
            vec![
                "Wensheng Lu".to_string(),
                "Keyu Chen".to_string(),
                "Ruizhi Qiao".to_string(),
                "Xing Sun".to_string(),
            ]
        );
        // Odd count: place the First Middle Last triple on the best middle token.
        let with_middle = split_author_names("Yuhao Wang Ruiyang Ren Wayne Xin Zhao Hua Wu");
        assert_eq!(
            with_middle,
            vec![
                "Yuhao Wang".to_string(),
                "Ruiyang Ren".to_string(),
                "Wayne Xin Zhao".to_string(),
                "Hua Wu".to_string(),
            ]
        );
        // Multi-space gaps (as Marker emits between authors) become commas.
        let cleaned = clean_author_byline_line(
            "Yuhao Wang  Ruiyang Ren  Yucheng Wang  Jing Liu  Wayne Xin Zhao  Hua Wu  Haifeng Wang",
        );
        assert_eq!(
            split_author_names(&cleaned),
            vec![
                "Yuhao Wang".to_string(),
                "Ruiyang Ren".to_string(),
                "Yucheng Wang".to_string(),
                "Jing Liu".to_string(),
                "Wayne Xin Zhao".to_string(),
                "Hua Wu".to_string(),
                "Haifeng Wang".to_string(),
            ]
        );
    }

    #[test]
    fn test_normalize_heading_text_and_frontmatter_stop() {
        assert_eq!(normalize_heading_text("#### **Abstract**"), "abstract");
        assert_eq!(
            normalize_heading_text("## **1 Introduction**"),
            "1 introduction"
        );
        assert_eq!(
            normalize_heading_text("- **Date:** Sep 15, 2025"),
            "date: sep 15, 2025"
        );
        assert_eq!(
            normalize_heading_text("- **Correspondence:** Ruizhi.Qiao@tencent.com"),
            "correspondence: ruizhi.qiao@tencent.com"
        );
        assert!(is_frontmatter_section_stop("#### **Abstract**"));
        assert!(is_frontmatter_section_stop("## **1 Introduction**"));
        assert!(is_frontmatter_section_stop("- **Date:** Sep 15, 2025"));
        assert!(is_frontmatter_section_stop("#### Introduction"));
        assert!(is_frontmatter_section_stop(
            "*Abstract*—Concerns regarding the propensity of LLMs"
        ));
        assert!(is_frontmatter_section_stop(
            "- **Data:** <https://huggingface.co/datasets/Youtu-RAG/HiCBench>"
        ));
        // Must not treat Elsevier journal titles as meta-stop bullets.
        assert!(!is_frontmatter_section_stop(
            "# Data & Knowledge Engineering"
        ));
        assert!(!is_frontmatter_section_stop(
            "Yuhao Wang $^{1}$ Ruiyang Ren $^{1}$"
        ));
        assert!(!is_frontmatter_section_stop(
            "Wensheng Lu * 1 Keyu Chen * 1 Ruizhi Qiao"
        ));
    }

    #[test]
    fn test_strip_markdown_links() {
        assert_eq!(
            strip_markdown_links("[Sebastian Farquhar](#page-1-0)"),
            "Sebastian Farquhar"
        );
        assert_eq!(
            strip_markdown_links("[Name 1](#link1), [Name 2](http://link2)"),
            "Name 1, Name 2"
        );
    }

    #[test]
    fn test_clean_reference_text_elsevier_refhub_nested_parens() {
        // Nested (26) in Elsevier refhub URLs must not leave "00077-7/sb2)" residue.
        let raw = "X. Guan, Mitigating large language model [hallucinations](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb2) via autonomous knowledge graph-based retrofitting, in: Proceedings of the AAAI, 2024, pp. [18126–18134.](http://refhub.elsevier.com/S0169-023X(26)00077-7/sb2)";
        let cleaned = clean_reference_text(raw);
        assert!(
            cleaned.contains("hallucinations via autonomous"),
            "expected mid-title word restored, got: {cleaned}"
        );
        assert!(
            !cleaned.contains("00077-7"),
            "refhub residue must be stripped, got: {cleaned}"
        );
        assert!(
            !cleaned.contains("refhub"),
            "refhub URL must not remain, got: {cleaned}"
        );
        // DOI / arXiv markdown links are preserved for downstream extractors.
        let arxiv = "Smith, Title, 2019, arXiv preprint [arXiv:1909.04164](http://arxiv.org/abs/1909.04164)";
        let cleaned_arxiv = clean_reference_text(arxiv);
        assert!(
            cleaned_arxiv.contains("arxiv.org") || cleaned_arxiv.contains("arXiv:1909.04164"),
            "arxiv link should be kept: {cleaned_arxiv}"
        );
    }

    #[test]
    fn test_map_markdown_links_balanced_parens() {
        let s = map_markdown_links(
            "see [foo](http://example.com/a(b)c) and [bar](#x)",
            |text, _url| text.to_string(),
        );
        assert_eq!(s, "see foo and bar");
    }

    #[test]
    fn test_strip_author_footnote_markers() {
        assert_eq!(strip_author_footnote_markers("Name<sup>1</sup>"), "Name");
        assert_eq!(strip_author_footnote_markers("Name [\\*1]"), "Name ");
        assert_eq!(strip_author_footnote_markers("Name [a]"), "Name ");
        assert_eq!(strip_author_footnote_markers("Name \\*"), "Name ");
        assert_eq!(strip_author_footnote_markers("Name †"), "Name ");
        assert_eq!(strip_author_footnote_markers("Name‡"), "Name");
    }

    #[test]
    fn test_is_affiliation_or_noise_line() {
        assert!(is_affiliation_or_noise_line("1 University of Oxford"));
        assert!(is_affiliation_or_noise_line(
            "Department of Computer Science"
        ));
        assert!(is_affiliation_or_noise_line("foo@bar.com"));
        assert!(!is_affiliation_or_noise_line(
            "Sebastian Farquhar, Jannik Kossen"
        ));
    }

    #[test]
    fn test_extract_doi() {
        assert_eq!(
            extract_doi("doi:10.1038/s41586-020-1234-y."),
            Some("10.1038/s41586-020-1234-y".to_string())
        );
        assert_eq!(
            extract_doi("Available at https://doi.org/10.1016/j.cell.2021.01.001;"),
            Some("10.1016/j.cell.2021.01.001".to_string())
        );
        assert_eq!(
            extract_doi("DOI (10.5555/3295222.3295349)"),
            Some("10.5555/3295222.3295349".to_string())
        );
        assert_eq!(extract_doi("No DOI here"), None);
    }

    #[test]
    fn test_extract_arxiv_id() {
        assert_eq!(
            extract_arxiv_id("See arXiv:1706.03762v1 for details."),
            Some("arXiv:1706.03762v1".to_string())
        );
        assert_eq!(
            extract_arxiv_id("Paper 1706.03762 was published in 2017."),
            Some("1706.03762".to_string())
        );
        assert_eq!(
            extract_arxiv_id("arxiv:math/0405001"),
            Some("arxiv:math/0405001".to_string())
        );
        assert_eq!(extract_arxiv_id("No arxiv here"), None);
    }

    #[test]
    fn test_extract_openreview_id() {
        assert_eq!(
            extract_openreview_id("https://openreview.net/forum?id=uccHPGDlao"),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(
            extract_openreview_id("https://openreview.net/pdf?id=uccHPGDlao"),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(
            extract_openreview_id("URL <https://openreview.net/forum?id=uccHPGDlao>."),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(
            extract_openreview_id("openreview:uccHPGDlao"),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(
            extract_openreview_id("OpenReview:uccHPGDlao"),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(
            extract_openreview_id("uccHPGDlao"),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(
            extract_openreview_id("Note ID uccHPGDlao"),
            Some("uccHPGDlao".to_string())
        );
        assert_eq!(extract_openreview_id("No openreview here"), None);
    }

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year("Vaswani et al. (2017)"), Some(2017));
        assert_eq!(extract_year("Published in [2024]."), Some(2024));
        assert_eq!(extract_year("In 1799 early work started"), None); // outside 1800-2030
        assert_eq!(extract_year("Future year 2050"), None);
    }

    #[test]
    fn test_extract_quoted_title() {
        assert_eq!(
            extract_quoted_title("Author et al. \"Attention is all you need.\" NeurIPS"),
            Some("Attention is all you need.".to_string())
        );
        assert_eq!(
            extract_quoted_title(
                "Author et al. “BERT: Pre-training of Deep Bidirectional Transformers” NAACL"
            ),
            Some("BERT: Pre-training of Deep Bidirectional Transformers".to_string())
        );
        assert_eq!(extract_quoted_title("Unquoted title here"), None);
    }

    #[test]
    fn test_is_reference_heading() {
        assert!(is_reference_heading("# References"));
        assert!(is_reference_heading("## Bibliography"));
        assert!(is_reference_heading("Literature Cited"));
        assert!(is_reference_heading("8. References"));
        assert!(is_reference_heading("## 10. References and Notes"));
        assert!(is_reference_heading("## **References**"));
        assert!(is_reference_heading("# REFERENCES"));
        assert!(is_reference_heading("## REFERENCES"));
        assert!(!is_reference_heading("# Introduction"));
        assert!(!is_reference_heading("Related Work"));
    }

    #[test]
    fn test_is_non_ref_heading() {
        assert!(is_non_ref_heading("Appendix"));
        assert!(is_non_ref_heading("# Appendix A"));
        assert!(is_non_ref_heading("Author contributions"));
        assert!(is_non_ref_heading("Acknowledgements"));
        assert!(is_non_ref_heading("Figures"));
        assert!(is_non_ref_heading("Tables"));
        assert!(is_non_ref_heading("Supplementary"));
        assert!(!is_non_ref_heading("# References"));
    }

    #[test]
    fn test_is_reference_entry_start() {
        assert!(is_reference_entry_start("[1] Vaswani et al."));
        assert!(is_reference_entry_start("1. Vaswani et al."));
        assert!(is_reference_entry_start(
            "[Vaswani 2017] Attention is all you need."
        ));
        assert!(is_reference_entry_start("(1) Shannon, C. E."));
        assert!(is_reference_entry_start(
            "- <span id=\"page-10-0\"></span>[1] Patrick Lewis et al."
        ));
        assert!(is_reference_entry_start(
            "- <span id=\"page-6-0\"></span>Saurav Kadavath et al."
        ));
        assert!(is_reference_entry_start("- Asai, A.; Wu, Z.; ..."));
        assert!(is_reference_entry_start(
            "<span id=\"page-8-4\"></span>Ebtesam Almazrouei et al."
        ));
        // Span stripped by splitter: dual-author + year must still start an entry
        assert!(is_reference_entry_start(
            "Amos Azaria and Tom Mitchell. 2023. The internal state of an llm knows when its lying."
        ));
        assert!(is_reference_entry_start(
            "Ben Wang and Aran Komatsuzaki. 2021. GPT-J-6B: A 6 Billion Parameter Autoregressive Language Model."
        ));
        assert!(is_reference_entry_start(
            "Andrey Malinin and Mark Gales. 2020. Uncertainty estimation in autoregressive structured prediction."
        ));
        // Unprefixed et al. body citations must still not start an entry
        assert!(!is_reference_entry_start(
            "Vaswani et al. (2017) Attention is all you need."
        ));
    }

    #[test]
    fn test_extract_latex_metadata_comment() {
        assert_eq!(
            extract_latex_metadata_comment(
                "% metadata: title = \"My Paper\", author = \"Jane Doe\""
            ),
            Some("title = \"My Paper\", author = \"Jane Doe\"".to_string())
        );
        assert_eq!(
            extract_latex_metadata_comment("  %metadata: key=value"),
            Some("key=value".to_string())
        );
        assert_eq!(
            extract_latex_metadata_comment("% Regular LaTeX comment"),
            None
        );
    }

    #[test]
    fn test_extract_reference_venue() {
        assert_eq!(
            extract_reference_venue("Published in Nature, vol 580, 2024"),
            Some("Nature".to_string())
        );
        assert_eq!(
            extract_reference_venue(
                "In Proceedings of the 62nd Annual Meeting of the Association for Computational Linguistics"
            ),
            Some("Association for Computational Linguistics".to_string())
        );
        assert_eq!(
            extract_reference_venue("arXiv preprint arXiv:2405.12345, 2024"),
            Some("arXiv".to_string())
        );
    }
}
