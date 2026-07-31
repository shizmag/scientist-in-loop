use xberg::{ExtractInput, ExtractionConfig, LlmConfig, StructuredExtractionConfig, extract};

#[tokio::main]
async fn main() {
    let schema_json = serde_json::json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" }
        }
    });

    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: schema_json,
            schema_name: "Test".to_string(),
            schema_description: None,
            strict: true,
            prompt: None,
            llm: LlmConfig::default(),
        }),
        ..Default::default()
    };
    
    let path = "/Users/vladimirkasterin/articles/entropy_framework/knowledge graph.pdf";
    let input = ExtractInput::from_uri(path);
    match extract(input, &config).await {
        Ok(res) => println!("Success: {:?}", res.results.first().map(|d| &d.content)),
        Err(e) => println!("Error: {:?}", e),
    }
}
