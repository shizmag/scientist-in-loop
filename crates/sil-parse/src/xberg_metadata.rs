//! Structured metadata extraction using the `xberg` crate.

use camino::Utf8Path;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::path::Path;
use xberg::{ExtractInput, ExtractionConfig, LlmConfig, StructuredExtractionConfig, extract};

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

/// Asynchronously extract structured metadata from a PDF file using `xberg`.
///
/// Sets model cache directory to `/Volumes/happy-disk/models/xberg/huggingface`
/// and ensures the path is created prior to execution.
pub async fn extract_metadata(
    file_path: &Path,
) -> std::result::Result<DocumentMetadata, anyhow::Error> {
    let cache_dir = Path::new("/Volumes/happy-disk/models/xberg/huggingface");
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

    let schema = schema_for!(DocumentMetadata);
    let schema_json = serde_json::to_value(&schema)?;

    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: schema_json,
            schema_name: "DocumentMetadata".to_string(),
            schema_description: Some(
                "Academic document metadata extraction (title, authors, citations)".to_string(),
            ),
            strict: true,
            prompt: None,
            llm: LlmConfig::default(),
        }),
        ..Default::default()
    };

    let path_str = file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 file path: {:?}", file_path))?;

    let input = ExtractInput::from_uri(path_str);
    let result = extract(input, &config)
        .await
        .map_err(|e| anyhow::anyhow!("xberg extraction failed: {:?}", e))?;

    let content = result
        .results
        .first()
        .map(|doc| doc.content.as_str())
        .ok_or_else(|| anyhow::anyhow!("xberg returned no document results"))?;

    let metadata: DocumentMetadata = serde_json::from_str(content).map_err(|e| {
        anyhow::anyhow!("Failed to deserialize xberg metadata JSON: {e}. Content was: {content}")
    })?;

    Ok(metadata)
}

/// Helper wrapper taking Utf8Path.
pub async fn extract_metadata_utf8(
    file_path: &Utf8Path,
) -> std::result::Result<DocumentMetadata, anyhow::Error> {
    extract_metadata(file_path.as_std_path()).await
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
