use assert_cmd::Command;
use camino::Utf8PathBuf;
use sil_core::NullUi;
use sil_db::SilDb;
use sil_parse::{StubMarkerRunner, parse_one, write_fixture_pdf};
use tempfile::tempdir;

#[test]
fn test_references_pipeline_e2e() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("attention.pdf");
    write_fixture_pdf(&pdf_path).unwrap();
    let pdf_utf8 = Utf8PathBuf::from_path_buf(pdf_path).unwrap();

    let db_path = dir.path().join("db.sqlite");
    let db_utf8 = Utf8PathBuf::from_path_buf(db_path).unwrap();
    let db = SilDb::open(&db_utf8).unwrap();
    let ui = NullUi::new();

    let markdown = r#"
# Attention Is All You Need

Abstract text...

## References
[1] Vaswani, A., et al. "Attention is all you need." NeurIPS, 2017. doi:10.5555/3295222.3295349
[2] Devlin, J., et al. "BERT: Pre-training of Deep Bidirectional Transformers." NAACL, 2019.
"#;

    let runner = StubMarkerRunner {
        content: markdown.to_string(),
    };

    // 1. Parse PDF -> extract references -> store in SQLite DB
    let result = parse_one(&pdf_utf8, &db, &runner, &ui).unwrap();
    assert!(result.document.parsed);
    assert!(result.document.references_text.is_some());

    // 2. Query references table
    let refs = db.get_references_for_source(&result.document.id).unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].ref_index, 1);
    assert_eq!(refs[0].year, Some(2017));
    assert_eq!(refs[0].title.as_deref(), Some("Attention is all you need."));

    // 3. Perform FTS Search on extracted references
    let hits = db.search_references("Bidirectional Transformers", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].ref_index, 2);

    // 4. Verify `sil cite` suggestion from extracted reference title
    let suggestion = sil_core::suggest_from_reference_entry(&hits[0]);
    assert!(suggestion.cite_command.contains("\\cite{"));
    assert!(suggestion.bibtex.contains("BERT: Pre-training of Deep Bidirectional Transformers."));
}
