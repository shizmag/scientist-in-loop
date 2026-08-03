use camino::Utf8PathBuf;
use serde_json::json;
use sil_core::{SourceDocument, SourceKind};
use sil_parse::hydrate_source_document_metadata;

#[test]
fn generate_candidate_extractions_for_golden_dataset() {
    let dataset_dir = Utf8PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden_dataset"
    ));
    let fixtures_dir = dataset_dir.join("fixtures");
    if !fixtures_dir.exists() {
        return;
    }

    let out_dir = Utf8PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/candidate_extractions"
    ));
    std::fs::create_dir_all(&out_dir).unwrap();

    let entries = std::fs::read_dir(&fixtures_dir).unwrap();
    let mut count = 0;

    for entry in entries {
        let entry = entry.unwrap();
        let fpath = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
        if !fpath.is_dir() {
            continue;
        }

        let stem = fpath.file_name().unwrap();
        let content_path = fpath.join("content.md");
        if !content_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&content_path).unwrap();
        let mut doc = SourceDocument::new(Utf8PathBuf::from(format!("{stem}.md")));
        doc.kind = SourceKind::Markdown;

        hydrate_source_document_metadata(&mut doc, &content, content_path.as_path());

        if doc.references_text.is_none() {
            doc.references_text = sil_parse::references::extract_references_block(&content);
        }

        let raw_refs = doc.references_text.as_ref().map(|b| {
            sil_parse::references::parse_reference_entries(&doc.id, b)
        }).unwrap_or_default();

        let refs_json: Vec<_> = raw_refs
            .iter()
            .map(|r| {
                json!({
                    "ref_index": r.ref_index,
                    "raw_text": r.raw_text,
                    "title": r.title,
                    "authors": r.authors,
                    "year": r.year,
                    "doi": r.doi,
                    "arxiv_id": r.arxiv_id,
                    "venue": r.venue
                })
            })
            .collect();

        let extraction = json!({
            "source": {
                "title": doc.title,
                "authors": doc.authors,
                "year": doc.year,
                "doi": doc.doi,
                "venue": doc.venue
            },
            "references": refs_json
        });

        let out_path = out_dir.join(format!("{stem}.json"));
        std::fs::write(&out_path, serde_json::to_string_pretty(&extraction).unwrap()).unwrap();
        count += 1;
    }

    println!("Emitted {count} candidate extractions to {out_dir}");
}
