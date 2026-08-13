use crate::app::source_badges::{
    derive_source_badges, extract_draft_cite_keys, BibEntry, SourceBadges, SourceRecord,
};
use camino::Utf8PathBuf;

#[test]
fn test_unparsed_not_in_bib_not_cited() {
    let source = SourceRecord::new(Utf8PathBuf::from("sources/raw_paper.pdf"));
    assert!(!source.parsed);

    let bib_entries: Vec<BibEntry> = vec![];
    let draft_tex = "\\section{Introduction}\nNo citations yet.";

    let badges = SourceBadges::derive(&source, &bib_entries, draft_tex);
    assert_eq!(
        badges,
        SourceBadges {
            parsed: false,
            in_bib: false,
            cited: false,
        }
    );
    assert_eq!(badges.format_badge(), "[unparsed]");
}

#[test]
fn test_parsed_in_bib_by_doi_not_cited() {
    let mut source = SourceRecord::new(Utf8PathBuf::from("sources/attention.pdf"));
    source.parsed = true;
    source.doi = Some("10.1000/182".to_string());
    source.title = Some("Attention Is All You Need".to_string());

    let bib_entry = BibEntry {
        cite_key: Some("vaswani2017attention".to_string()),
        doi: Some("10.1000/182".to_string()),
        title: Some("Attention Is All You Need".to_string()),
        arxiv_id: None,
        is_incomplete: false,
    };
    let bib_entries = vec![bib_entry];
    let draft_tex = "\\section{Introduction}\nWe describe our approach without citing anything.";

    let badges = SourceBadges::derive(&source, &bib_entries, draft_tex);
    assert_eq!(
        badges,
        SourceBadges {
            parsed: true,
            in_bib: true,
            cited: false,
        }
    );
    assert_eq!(badges.format_badge(), "[parsed · in bib]");
}

#[test]
fn test_parsed_in_bib_cited_in_draft() {
    let mut source = SourceRecord::new(Utf8PathBuf::from("sources/attention.pdf"));
    source.parsed = true;
    source.doi = Some("10.1000/182".to_string());
    source.title = Some("Attention Is All You Need".to_string());

    let bib_entry = BibEntry {
        cite_key: Some("vaswani2017".to_string()),
        doi: Some("10.1000/182".to_string()),
        title: Some("Attention Is All You Need".to_string()),
        arxiv_id: None,
        is_incomplete: false,
    };
    let bib_entries = vec![bib_entry];
    let draft_tex = "\\section{Related Work}\nAs shown in \\cite{vaswani2017}, self-attention is effective.";

    let badges = SourceBadges::derive(&source, &bib_entries, draft_tex);
    assert_eq!(
        badges,
        SourceBadges {
            parsed: true,
            in_bib: true,
            cited: true,
        }
    );
    assert_eq!(badges.format_badge(), "[parsed · in bib · cited]");
}

#[test]
fn test_multi_key_cite_counts_for_both() {
    let mut source_a = SourceRecord::new(Utf8PathBuf::from("sources/a.pdf"));
    source_a.parsed = true;

    let mut source_b = SourceRecord::new(Utf8PathBuf::from("sources/b.pdf"));
    source_b.parsed = true;

    let bib_a = BibEntry {
        cite_key: Some("a".to_string()),
        ..Default::default()
    };
    let bib_b = BibEntry {
        cite_key: Some("b".to_string()),
        ..Default::default()
    };
    let bib_entries = vec![bib_a, bib_b];

    let draft_tex = "Prior work \\cite{a, b} established the foundation.";

    let badges_a = SourceBadges::derive(&source_a, &bib_entries, draft_tex);
    let badges_b = derive_source_badges(&source_b, &bib_entries, draft_tex);

    assert!(badges_a.parsed);
    assert!(badges_a.in_bib);
    assert!(badges_a.cited);
    assert_eq!(badges_a.format_badge(), "[parsed · in bib · cited]");

    assert!(badges_b.parsed);
    assert!(badges_b.in_bib);
    assert!(badges_b.cited);
    assert_eq!(badges_b.format_badge(), "[parsed · in bib · cited]");
}

#[test]
fn test_parsed_not_in_bib_not_cited() {
    let mut source = SourceRecord::new(Utf8PathBuf::from("sources/local_notes.md"));
    source.parsed = true;

    let bib_entries: Vec<BibEntry> = vec![];
    let draft_tex = "Draft content without citations.";

    let badges = SourceBadges::derive(&source, &bib_entries, draft_tex);
    assert_eq!(
        badges,
        SourceBadges {
            parsed: true,
            in_bib: false,
            cited: false,
        }
    );
    assert_eq!(badges.format_badge(), "[parsed]");
}

#[test]
fn test_cite_macro_variants_and_comment_handling() {
    let tex = r#"
\section{Methods}
Standard citation \cite{key_standard}.
Parenthetical with options \citep[see][p. 10]{key_p1, key_p2}.
Textual citation \citet{key_text}.
Author citation \citeauthor{key_author}.
Autocite \autocite{key_auto}.
% \cite{commented_key}
Inline comment \cite{key_active} % \cite{inline_comment_key}
Line with escaped percent 100\% \cite{key_escaped_percent}.
"#;

    let keys = extract_draft_cite_keys(tex);
    assert!(keys.contains("key_standard"));
    assert!(keys.contains("key_p1"));
    assert!(keys.contains("key_p2"));
    assert!(keys.contains("key_text"));
    assert!(keys.contains("key_author"));
    assert!(keys.contains("key_auto"));
    assert!(keys.contains("key_active"));
    assert!(keys.contains("key_escaped_percent"));

    assert!(!keys.contains("commented_key"));
    assert!(!keys.contains("inline_comment_key"));
}

#[test]
fn test_doi_normalization_match() {
    let mut source = SourceRecord::new(Utf8PathBuf::from("paper.pdf"));
    source.parsed = true;
    source.doi = Some("https://doi.org/10.1145/3377811.3380364".to_string());

    let bib_entry = BibEntry {
        cite_key: Some("chi2020".to_string()),
        doi: Some("10.1145/3377811.3380364".to_string()),
        ..Default::default()
    };

    let draft_tex = "We refer to \\citep{chi2020}.";
    let badges = SourceBadges::derive(&source, &[bib_entry], draft_tex);
    assert!(badges.parsed);
    assert!(badges.in_bib);
    assert!(badges.cited);
    assert_eq!(badges.format_badge(), "[parsed · in bib · cited]");
}
