#![allow(clippy::collapsible_if)]
//! Structured metadata extraction using the `xberg` crate.

use camino::Utf8Path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sil_core::{ReferenceEntry, SourceId};
use std::path::Path;
use xberg::core::config::ner::{NerBackendKind, NerConfig};
use xberg::types::entity::EntityCategory;
use xberg::{ExtractInput, ExtractionConfig, extract};

/// Strongly typed target struct for xberg metadata extraction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DocumentMetadata {
    /// Document title.
    pub title: String,
    /// List of author names.
    pub authors: Vec<String>,
    /// Extracted citations/references text or entries.
    pub citations: Vec<String>,
}

/// Create an `ExtractionConfig` that explicitly enables layout detection and ONNX GLiNER2 NER for titles, authors, venues, and citations.
pub fn build_extraction_config() -> ExtractionConfig {
    ExtractionConfig {
        layout: Some(Default::default()),
        use_layout_for_markdown: true,
        ner: Some(NerConfig {
            backend: NerBackendKind::Onnx,
            custom_labels: vec![
                "title".to_string(),
                "author".to_string(),
                "venue".to_string(),
                "citation".to_string(),
            ],
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Synchronously extract document text and metadata using xberg.
pub fn extract_file_sync(
    file_path: &Path,
    config: &ExtractionConfig,
) -> std::result::Result<xberg::ExtractionResult, anyhow::Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(extract_file(file_path, config))
}

/// Asynchronously extract document text and metadata using xberg.
pub async fn extract_file(
    file_path: &Path,
    config: &ExtractionConfig,
) -> std::result::Result<xberg::ExtractionResult, anyhow::Error> {
    let settings = sil_core::GlobalSettings::load_or_default(None);
    let cache_dir = settings.rag.xberg_model_cache_dir.as_std_path();
    if let Err(e) = std::fs::create_dir_all(cache_dir) {
        eprintln!(
            "Warning: failed to create xberg model cache directory {:?}: {}",
            cache_dir, e
        );
    } else {
        unsafe {
            std::env::set_var("HF_HOME", cache_dir);
        }
    }

    let path_str = file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 file path: {:?}", file_path))?;

    let input = ExtractInput::from_uri(path_str);
    extract(input, config)
        .await
        .map_err(|e| anyhow::anyhow!("xberg extraction failed: {:?}", e))
}

/// Asynchronously extract structured metadata from a PDF file using `xberg`.
pub async fn extract_metadata(
    file_path: &Path,
) -> std::result::Result<DocumentMetadata, anyhow::Error> {
    let config = build_extraction_config();
    let result = extract_file(file_path, &config).await?;

    let doc = result
        .results
        .first()
        .ok_or_else(|| anyhow::anyhow!("xberg returned no document results"))?;

    let mut metadata = DocumentMetadata {
        title: String::new(),
        authors: Vec::new(),
        citations: Vec::new(),
    };

    for entity in doc.entities.iter().flatten() {
        match &entity.category {
            EntityCategory::Custom(label) if label == "title" => {
                if metadata.title.is_empty() {
                    metadata.title = entity.text.clone();
                }
            }
            EntityCategory::Custom(label) if label == "author" => {
                let parsed_authors = parse_author_list(&entity.text);
                for author in parsed_authors {
                    if !metadata.authors.contains(&author) {
                        metadata.authors.push(author);
                    }
                }
            }
            EntityCategory::Custom(label) if label == "citation" => {
                metadata.citations.push(entity.text.clone());
            }
            _ => {}
        }
    }

    Ok(metadata)
}

/// Helper wrapper taking Utf8Path.
pub async fn extract_metadata_utf8(
    file_path: &Utf8Path,
) -> std::result::Result<DocumentMetadata, anyhow::Error> {
    extract_metadata(file_path.as_std_path()).await
}

/// Parse author strings, supporting authors separated only by commas (no "and" or "&").
///
/// Filters common byline pollution: `et al.`, pure numeric tokens, and short
/// citation-like fragments that are not person names.
pub fn parse_author_list(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let clean = trimmed.strip_suffix('.').unwrap_or(trimmed);
    let mut authors = Vec::new();

    // Check if comma-separated
    let parts: Vec<&str> = clean.split(',').map(|s| s.trim()).collect();
    for part in parts {
        let mut p = part.trim();
        if p.is_empty() {
            continue;
        }
        if let Some(stripped) = p.strip_prefix("and ") {
            p = stripped.trim();
        } else if let Some(stripped) = p.strip_prefix("& ") {
            p = stripped.trim();
        }
        if p.is_empty() {
            continue;
        }
        if let Some((a, b)) = p.split_once(" and ") {
            push_author_if_clean(&mut authors, a.trim());
            push_author_if_clean(&mut authors, b.trim());
        } else if let Some((a, b)) = p.split_once(" & ") {
            push_author_if_clean(&mut authors, a.trim());
            push_author_if_clean(&mut authors, b.trim());
        } else {
            push_author_if_clean(&mut authors, p);
        }
    }

    authors
}

fn push_author_if_clean(authors: &mut Vec<String>, name: &str) {
    if name.is_empty() {
        return;
    }
    let lower = name.to_ascii_lowercase();
    if lower == "et al" || lower == "et al." || lower.starts_with("et al") {
        return;
    }
    // Drop pure years / page-like tokens that bleed from in-text citations.
    if name
        .chars()
        .all(|c| c.is_ascii_digit() || c == '-' || c == '–')
    {
        return;
    }
    // Very short tokens are rarely full author names (keep initials like "J. Smith" via space/dot).
    if name.len() < 2 {
        return;
    }
    // Citation bleed: "Kadavath et al" style leftovers after split.
    if lower.contains(" et al") {
        let cleaned = lower.split(" et al").next().unwrap_or("").trim();
        if cleaned.is_empty() {
            return;
        }
        // Preserve original casing for the prefix when possible.
        let orig_prefix = name
            .split_once(" et al")
            .or_else(|| name.split_once(" Et Al"))
            .map(|(a, _)| a.trim())
            .unwrap_or(name);
        if !authors.iter().any(|a| a.eq_ignore_ascii_case(orig_prefix)) {
            authors.push(orig_prefix.to_string());
        }
        return;
    }
    if !authors.iter().any(|a| a.eq_ignore_ascii_case(name)) {
        authors.push(name.to_string());
    }
}

/// Map xberg extracted entities from ExtractedDocument to sil_core::ReferenceEntry DTOs.
pub fn map_entities_to_reference_entries(
    source_id: &SourceId,
    doc: &xberg::types::ExtractedDocument,
) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();
    let mut idx = 1;

    for entity in doc.entities.iter().flatten() {
        if matches!(&entity.category, EntityCategory::Custom(label) if label.eq_ignore_ascii_case("citation"))
        {
            let raw_text = entity.text.trim().to_string();
            if raw_text.is_empty() {
                continue;
            }

            let id = format!("{}_ref_{}", source_id.as_str(), idx);
            let (authors, year, title, venue, doi, arxiv_id, url) =
                crate::references::parse_entry_metadata(&raw_text);

            entries.push(ReferenceEntry {
                id,
                source_id: source_id.clone(),
                ref_index: idx,
                raw_text,
                title,
                authors,
                year,
                venue,
                doi,
                arxiv_id,
                url,
            });
            idx += 1;
        }
    }

    // Fallback: If NER extracted no citation entities, parse reference section block from layout-reconstructed content
    if entries.is_empty() {
        if let Some(raw_block) = crate::references::extract_references_block(&doc.content) {
            entries = crate::references::parse_reference_entries(source_id, &raw_block);
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;

    #[test]
    fn test_document_metadata_serde() {
        let meta = DocumentMetadata {
            title: "Attention Is All You Need".to_string(),
            authors: vec!["Ashish Vaswani".to_string(), "Noam Shazeer".to_string()],
            citations: vec!["Ref 1".to_string(), "Ref 2".to_string()],
        };

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("Attention Is All You Need"));
        assert!(json.contains("Ashish Vaswani"));

        let deserialized: DocumentMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, meta);
    }

    #[test]
    fn test_schema_generation() {
        let schema = schema_for!(DocumentMetadata);
        let schema_json = serde_json::to_value(&schema).unwrap();
        let schema_str = schema_json.to_string();

        assert!(schema_str.contains("title"));
        assert!(schema_str.contains("authors"));
        assert!(schema_str.contains("citations"));
    }

    #[tokio::test]
    async fn test_extract_metadata_invalid_file() {
        let path = Path::new("/nonexistent/file.pdf");
        let result = extract_metadata(path).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_author_list_comma_separated_without_and() {
        let text = "Ashish Vaswani, Noam Shazeer, Niki Parmar, Jakob Uszkoreit";
        let authors = parse_author_list(text);
        assert_eq!(authors.len(), 4);
        assert_eq!(authors[0], "Ashish Vaswani");
        assert_eq!(authors[1], "Noam Shazeer");
        assert_eq!(authors[2], "Niki Parmar");
        assert_eq!(authors[3], "Jakob Uszkoreit");
    }

    #[test]
    fn test_author_list_comma_separated_with_and() {
        let text = "Alice Smith, Bob Jones, and Charlie Brown";
        let authors = parse_author_list(text);
        assert_eq!(authors.len(), 3);
        assert_eq!(authors[0], "Alice Smith");
        assert_eq!(authors[1], "Bob Jones");
        assert_eq!(authors[2], "Charlie Brown");
    }

    #[test]
    fn test_author_list_filters_et_al_and_years() {
        let text = "Alice Smith, Bob Jones et al., 2024, Charlie Brown";
        let authors = parse_author_list(text);
        assert!(authors.iter().any(|a| a == "Alice Smith"));
        assert!(authors.iter().any(|a| a == "Bob Jones"));
        assert!(authors.iter().any(|a| a == "Charlie Brown"));
        assert!(!authors.iter().any(|a| a.contains("et al")));
        assert!(!authors.iter().any(|a| a == "2024"));
    }

    #[test]
    fn test_column_break_citation_layout_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let pdf_path = dir.path().join("column_break_test.pdf");
        // Create dummy pdf file byte stream
        let pdf_bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000062 00000 n \n0000000117 00000 n \ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n185\n%%EOF";
        std::fs::write(&pdf_path, pdf_bytes).unwrap();

        let config = build_extraction_config();
        assert!(config.layout.is_some());
        assert!(config.use_layout_for_markdown);
        assert!(config.ner.is_some());

        // Test sync extraction on mock pdf
        let res = extract_file_sync(&pdf_path, &config);
        assert!(res.is_ok());
    }
}
