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
        // Branch 1: numbered/bracketed entries, or "Surname," pattern (with optional - and <span> prefix)
        r"^\s*(?:-\s+)?(?:<span[^>]*>.*?</span>\s*)?(?:\[\d+\]|\(\d+\)|\d+[\.\)]|\([^\)]*\d{4}\)|\[[^\]]*\d{4}\]|[A-Z][a-z]+[,\;\:]\s+[A-Z])",
        r"|",
        // Branch 2: "- " and/or <span> prefixed "Name et al" entries (requires at least one list marker)
        r"^\s*(?:-\s+(?:<span[^>]*>.*?</span>\s*)?|<span[^>]*>.*?</span>\s*)[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\s+et\s+al",
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

/// Extract an arXiv identifier from text (e.g. `1706.03762` or `arxiv:1706.03762v1`).
pub fn extract_arxiv_id(text: &str) -> Option<String> {
    ARXIV_REGEX.find(text).map(|m| {
        m.as_str()
            .trim_end_matches(&['.', ',', ';', ')', ']'][..])
            .to_string()
    })
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
