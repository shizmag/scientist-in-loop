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
    Regex::new(r"(?i)^\s*#*\s*(?:\d+\.?)?\s*(appendix|author contributions|acknowledgements|acknowledgments|figures|tables|supplementary)\b").unwrap()
});

static REF_ENTRY_START_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:-\s+)?(?:<span[^>]*>.*?</span>\s*)?(?:\[\d+\]|\(\d+\)|\d+\.|\([^\)]*\d{4}\)|\[[^\]]*\d{4}\])|^\s*(?:-\s+)?[A-Z][a-z]+,\s+[A-Z]").unwrap()
});

static LATEX_METADATA_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*%\s*metadata:\s*(.+)$").unwrap());

static HTML_SPAN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<span[^>]*>(?:</span>)?").unwrap());

/// Strip HTML `<span...>` and `</span>` tags from text.
pub fn strip_html_spans(text: &str) -> String {
    HTML_SPAN_REGEX.replace_all(text, "").to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
