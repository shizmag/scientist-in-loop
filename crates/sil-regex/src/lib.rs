//! Centralized regular expressions and text pattern matchers for scientist-in-loop.

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

static YEAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(1[89]\d{2}|20[0-2]\d|2030)\b").unwrap());

static QUOTED_TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"["“]([^"”\r\n]{2,})[”"]"#).unwrap());

static REF_HEADING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*#*\s*(?:\d+\.?)?\s*(?:\*\*|__)?\s*(references|bibliography|literature cited|works cited|references and notes)(?:\*\*|__)?\b").unwrap()
});

static NON_REF_HEADING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*#*\s*(?:\d+\.?)?\s*(appendix|author contributions|acknowledgements|acknowledgments|figures|tables|supplementary|supplemental|ethics statement|declarations|competing interests|conflict of interest|about the authors|biography|author biographies)\b").unwrap()
});

static REF_ENTRY_START_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        // Branch 1: numbered/bracketed entries, "Surname," pattern, or "Firstname Surname," pattern (with optional -, *, • and <span> prefix)
        r"^\s*(?:[\-*•]\s+)?(?:<span[^>]*>.*?</span>\s*)?(?:\[\d+\]|\(\d+\)|\d+[\.\)]|\([^\)]*\d{4}\)|\[[^\]]*\d{4}\]|[A-Z][a-z]+[,\;\:]\s+[A-Z]|[A-Z][a-z]+(?:\s+[A-Z]\.|\s+[A-Z][a-z]+)+[,\;\.])",
        r"|",
        // Branch 2: "- " and/or <span> prefixed "Name et al" entries (requires at least one list marker)
        r"^\s*(?:[\-*•]\s+(?:<span[^>]*>.*?</span>\s*)?|<span[^>]*>.*?</span>\s*)[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\s+et\s+al",
    )).unwrap()
});

static LATEX_METADATA_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*%\s*metadata:\s*(.+)$").unwrap());

static HTML_SPAN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<span[^>]*>(?:</span>)?").unwrap());

static A_TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<a[^>]*>|</a>").unwrap());

static MD_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap());

static MD_LINK_WITH_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());

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
pub fn strip_markdown_links(text: &str) -> String {
    MD_LINK_REGEX.replace_all(text, "$1").to_string()
}

/// Strip author footnote markers like `<sup>...</sup>`, `[\*1]`, `[a]`, `\*`, `†`, etc.
pub fn strip_author_footnote_markers(text: &str) -> String {
    AUTHOR_FOOTNOTE_REGEX.replace_all(text, "").to_string()
}

/// Clean reference text: strips HTML span/a tags, markdown links (except DOI/arXiv), normalizes spaces, trims list prefixes.
pub fn clean_reference_text(text: &str) -> String {
    let mut cleaned = HTML_SPAN_REGEX.replace_all(text, "").to_string();
    cleaned = A_TAG_REGEX.replace_all(&cleaned, "").to_string();

    cleaned = MD_LINK_WITH_URL_REGEX
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let text_content = &caps[1];
            let url = &caps[2];
            if url.contains("10.") || url.contains("arxiv") || extract_arxiv_id(url).is_some() {
                caps[0].to_string()
            } else {
                text_content.to_string()
            }
        })
        .to_string();

    static MULTI_SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s{2,}").unwrap());
    cleaned = MULTI_SPACE_REGEX.replace_all(&cleaned, " ").to_string();

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
    let raw = clean.trim_start_matches('#').trim().trim_matches('*').trim_matches('_').trim();
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
        s = s[..idx].to_string();
    } else if is_affiliation_or_noise_line(&s) {
        return String::new();
    }

    // Strip TeX math footnote markers e.g. $^{1*\dagger}$, $^1$, $^{2\ddagger}$
    static TEX_MATH_NOISE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\$\^\{?[^}]*\}?\$|\$[^$]*\$|\^\{[^}]*\}|\^\[[^\]]*\]").unwrap()
    });
    s = TEX_MATH_NOISE_REGEX.replace_all(&s, "").to_string();

    // Strip HTML <sup>...</sup>
    s = strip_author_footnote_markers(&s);

    // Strip footnote/superscript characters
    static NOISE_CHARS: &[char] = &[
        '*', '⋈', '†', '‡', '§', '¶', '♯', '♠', '¹', '²', '³', '⁴', '⁵', '⁶',
        '⁷', 'ⁿ', '՞', 'ã', 'ゥ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    ];
    s = s.replace(NOISE_CHARS, "");

    // Strip IEEE badges
    s = IEEE_BADGE_REGEX.replace_all(&s, "").to_string();

    s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.trim_matches(|c: char| c == ',' || c == ';' || c == '-' || c.is_whitespace()).to_string()
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
                let clean_item = item.trim().trim_matches(|c: char| c == ',' || c == ';' || c.is_whitespace()).to_string();
                if !clean_item.is_empty() {
                    res.push(clean_item);
                }
            }
        }
    } else {
        // Line has no commas/semicolons: e.g. "Wensheng Lu Keyu Chen Ruizhi Qiao Xing Sun"
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut idx = 0;
        while idx < words.len() {
            let w1 = words[idx];
            if idx + 1 < words.len() {
                let w2 = words[idx + 1];
                let is_w1_cap = w1.chars().next().is_some_and(|c| c.is_uppercase());
                let is_w2_cap = w2.chars().next().is_some_and(|c| c.is_uppercase());
                if is_w1_cap && is_w2_cap {
                    // Check if 3rd word is middle initial or name e.g. "Wayne Xin Zhao"
                    if idx + 2 < words.len() {
                        let w3 = words[idx + 2];
                        let is_w3_cap = w3.chars().next().is_some_and(|c| c.is_uppercase());
                        // If w2 is a middle initial or short middle name and w3 is capitalized
                        if is_w3_cap && (w2.len() <= 3 || idx + 3 >= words.len() || !words[idx + 3].chars().next().is_some_and(|c| c.is_uppercase())) {
                            res.push(format!("{w1} {w2} {w3}"));
                            idx += 3;
                            continue;
                        }
                    }
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
    REF_ENTRY_START_REGEX.is_match(line)
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
