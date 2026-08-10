use super::*;
use std::time::Instant;

#[test]
fn test_clean_doi_str() {
    assert_eq!(
        clean_doi_str("doi:10.1038/s41586-020-1234-y"),
        "10.1038/s41586-020-1234-y"
    );
    assert_eq!(
        clean_doi_str("https://doi.org/10.1038/s41586-020-1234-y"),
        "10.1038/s41586-020-1234-y"
    );
    assert_eq!(
        clean_doi_str("http://doi.org/10.1038/s41586-020-1234-y"),
        "10.1038/s41586-020-1234-y"
    );
    assert_eq!(
        clean_doi_str("http://dx.doi.org/10.1000/182"),
        "10.1000/182"
    );
    assert_eq!(
        clean_doi_str("https://dx.doi.org/10.1000/182"),
        "10.1000/182"
    );
    assert_eq!(
        clean_doi_str("  10.1038/s41586-020-1234-y \n"),
        "10.1038/s41586-020-1234-y"
    );
    assert_eq!(clean_doi_str(""), "");
}

#[test]
fn test_check_doi_exists_empty() {
    assert!(!check_doi_exists("").unwrap());
    assert!(!check_doi_exists("   ").unwrap());
}

#[test]
fn test_enforce_api_ratelimit() {
    let start = Instant::now();
    enforce_api_ratelimit();
    enforce_api_ratelimit();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 200,
        "Expected at least ~200ms delay between ratelimited calls, got {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_error_categorization() {
    let not_found = ApiError::NotFound("DOI 10.0000/foo".into());
    assert_eq!(not_found.to_string(), "Not found: DOI 10.0000/foo");

    let rate_limited = ApiError::RateLimited("429 Too Many Requests".into());
    assert_eq!(rate_limited.to_string(), "Rate limited: 429 Too Many Requests");

    let net_err = ApiError::NetworkError("Connection refused".into());
    assert_eq!(net_err.to_string(), "Network error: Connection refused");

    let parse_err = ApiError::ParseError("Invalid JSON".into());
    assert_eq!(parse_err.to_string(), "Parse error: Invalid JSON");

    let invalid_id = ApiError::InvalidIdentifier("Malformed input".into());
    assert_eq!(invalid_id.to_string(), "Invalid identifier: Malformed input");
}

#[test]
fn test_title_similarity_function() {
    assert_eq!(
        title_similarity("Attention Is All You Need", "Attention Is All You Need"),
        1.0
    );
    assert_eq!(
        title_similarity("Attention Is All You Need!", "attention is all you need."),
        1.0
    );
    assert_eq!(title_similarity("", ""), 1.0);
    assert_eq!(title_similarity("Some Title", ""), 0.0);

    let sim = title_similarity(
        "Attention Is All You Need",
        "Attention Is All You Need for Deep Learning",
    );
    assert!(sim >= 0.60, "Expected sim >= 0.60, got {sim}");

    let low_sim = title_similarity("Attention Is All You Need", "Quantum Supremacy Processor");
    assert!(low_sim < 0.60, "Expected low_sim < 0.60, got {low_sim}");
}

#[test]
fn test_clean_arxiv_id_str() {
    assert_eq!(clean_arxiv_id_str("arxiv:2405.12345"), "2405.12345");
    assert_eq!(clean_arxiv_id_str("arXiv:2405.12345"), "2405.12345");
    assert_eq!(
        clean_arxiv_id_str("https://arxiv.org/abs/2405.12345v1"),
        "2405.12345v1"
    );
    assert_eq!(clean_arxiv_id_str(""), "");
}

#[test]
fn test_parse_crossref_item_json() {
    let json = serde_json::json!({
        "DOI": "10.1038/s41586-020-1234-y",
        "title": ["Quantum Supremacy in a Programmable Processor"],
        "author": [
            {"given": "Frank", "family": "Arute"},
            {"given": "Kunald", "family": "Arya"}
        ],
        "container-title": ["Nature"],
        "published": {
            "date-parts": [[2019, 10, 23]]
        },
        "abstract": "<jats:p>The promise of quantum computers...</jats:p>",
        "is-referenced-by-count": 1500,
        "URL": "https://doi.org/10.1038/s41586-020-1234-y",
        "link": [
            {"URL": "https://nature.com/articles/s41586-020-1234-y.pdf", "content-type": "application/pdf"}
        ]
    });

    let pub_item = parse_crossref_item(&json).expect("should parse valid crossref item");
    assert_eq!(pub_item.doi.as_deref(), Some("10.1038/s41586-020-1234-y"));
    assert_eq!(
        pub_item.title,
        "Quantum Supremacy in a Programmable Processor"
    );
    assert_eq!(pub_item.authors, "Frank Arute, Kunald Arya");
    assert_eq!(pub_item.journal, "Nature");
    assert_eq!(pub_item.year, Some(2019));
    assert_eq!(
        pub_item.abstract_text,
        "The promise of quantum computers..."
    );
    assert_eq!(pub_item.citation_count, Some(1500));
    assert_eq!(
        pub_item.pdf_url.as_deref(),
        Some("https://nature.com/articles/s41586-020-1234-y.pdf")
    );
}

#[test]
fn test_build_crossref_digest_url_parameters() {
    let url = build_crossref_digest_url("quantum computing", 10);
    assert!(url.starts_with("https://api.crossref.org/works"));
    assert!(url.contains("query=quantum%20computing"));
    assert!(url.contains("filter=type:journal-article"));
    assert!(url.contains("rows=10"));
    assert!(url.contains("sort=relevance"));
}

#[test]
fn test_fetch_work_by_arxiv_id_empty() {
    let res = fetch_work_by_arxiv_id("  ").unwrap();
    assert!(res.is_none());
}

#[test]
fn test_verify_doi_with_metadata_empty() {
    let res = verify_doi_with_metadata("").unwrap();
    assert_eq!(
        res,
        DoiMetadataResult {
            exists: false,
            title: None,
        }
    );

    let res_spaces = verify_doi_with_metadata("   ").unwrap();
    assert_eq!(
        res_spaces,
        DoiMetadataResult {
            exists: false,
            title: None,
        }
    );
}

#[test]
fn test_extract_title_from_crossref_json() {
    let json_with_title = serde_json::json!({
        "status": "ok",
        "message-type": "work",
        "message": {
            "title": ["Attention Is All You Need", "Secondary Title"],
            "DOI": "10.48550/arXiv.1706.03762"
        }
    });
    assert_eq!(
        doi::extract_title_from_crossref_json(&json_with_title),
        Some("Attention Is All You Need".to_string())
    );

    let json_empty_title_arr = serde_json::json!({
        "message": {
            "title": []
        }
    });
    assert_eq!(
        doi::extract_title_from_crossref_json(&json_empty_title_arr),
        None
    );

    let json_no_title = serde_json::json!({
        "message": {}
    });
    assert_eq!(doi::extract_title_from_crossref_json(&json_no_title), None);

    let json_no_message = serde_json::json!({});
    assert_eq!(
        doi::extract_title_from_crossref_json(&json_no_message),
        None
    );
}

#[test]
fn test_clean_openreview_id_str() {
    assert_eq!(
        clean_openreview_id_str("https://openreview.net/forum?id=uccHPGDlao"),
        "uccHPGDlao"
    );
    assert_eq!(
        clean_openreview_id_str("https://openreview.net/pdf?id=uccHPGDlao"),
        "uccHPGDlao"
    );
    assert_eq!(
        clean_openreview_id_str("openreview:uccHPGDlao"),
        "uccHPGDlao"
    );
    assert_eq!(clean_openreview_id_str("uccHPGDlao"), "uccHPGDlao");
    assert_eq!(clean_openreview_id_str(""), "");
}

#[test]
fn test_extract_title_from_openreview_note() {
    let note_v2 = serde_json::json!({
        "id": "uccHPGDlao",
        "content": {
            "title": {
                "value": "Judging LLM-as-a-judge with MT-bench"
            }
        }
    });
    assert_eq!(
        extract_title_from_openreview_note(&note_v2),
        Some("Judging LLM-as-a-judge with MT-bench".to_string())
    );

    let note_v1 = serde_json::json!({
        "id": "uccHPGDlao",
        "content": {
            "title": "Judging LLM-as-a-judge with MT-bench"
        }
    });
    assert_eq!(
        extract_title_from_openreview_note(&note_v1),
        Some("Judging LLM-as-a-judge with MT-bench".to_string())
    );

    let note_no_title = serde_json::json!({
        "id": "uccHPGDlao",
        "content": {}
    });
    assert_eq!(extract_title_from_openreview_note(&note_no_title), None);
}

#[test]
fn test_verify_openreview_with_metadata_empty() {
    let res = verify_openreview_with_metadata("").unwrap();
    assert_eq!(
        res,
        DoiMetadataResult {
            exists: false,
            title: None,
        }
    );

    let res_spaces = verify_openreview_with_metadata("   ").unwrap();
    assert_eq!(
        res_spaces,
        DoiMetadataResult {
            exists: false,
            title: None,
        }
    );
}

#[test]
fn test_verify_arxiv_with_metadata_empty() {
    let res = verify_arxiv_with_metadata("").unwrap();
    assert_eq!(
        res,
        DoiMetadataResult {
            exists: false,
            title: None,
        }
    );

    let res_spaces = verify_arxiv_with_metadata("   ").unwrap();
    assert_eq!(
        res_spaces,
        DoiMetadataResult {
            exists: false,
            title: None,
        }
    );
}


