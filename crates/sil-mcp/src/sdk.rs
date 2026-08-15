//! Official `rmcp` service implementation for the MCP stdio endpoint.

use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};

use crate::{
    McpContext,
    protocol::CallToolResult,
    tools::{call_tool_with_context, list_tools},
};

/// Resource URI for the project-level context snapshot.
pub const CONTEXT_RESOURCE: &str = "sil://project/context";
/// Resource URI for the draft manuscript.
pub const MANUSCRIPT_RESOURCE: &str = "sil://project/manuscript";
/// Resource URI for source files.
pub const SOURCES_RESOURCE: &str = "sil://project/sources";
/// Resource URI for review reports.
pub const REPORTS_RESOURCE: &str = "sil://project/reports";

/// `rmcp` server handler bound to one canonical D1 project context.
#[derive(Debug, Clone)]
pub struct SilMcpHandler {
    context: McpContext,
}

impl SilMcpHandler {
    /// Construct a handler that cannot access another project root.
    pub fn new(context: McpContext) -> Self {
        Self { context }
    }

    fn tools(&self) -> Vec<rmcp::model::Tool> {
        list_tools()
            .into_iter()
            .map(|tool| {
                let schema = serde_json::from_value(tool.input_schema.properties)
                    .unwrap_or_else(|_| serde_json::Map::new());
                let mut schema = schema;
                schema.insert(
                    "type".into(),
                    serde_json::json!(tool.input_schema.schema_type),
                );
                if !tool.input_schema.required.is_empty() {
                    schema.insert(
                        "required".into(),
                        serde_json::json!(tool.input_schema.required),
                    );
                }
                if let Some(variants) = action_variants(&tool.name) {
                    schema.insert("oneOf".into(), variants);
                }
                rmcp::model::Tool::new(tool.name, tool.description, schema)
            })
            .collect()
    }

    fn resources(&self) -> Vec<Resource> {
        vec![
            Resource::new(CONTEXT_RESOURCE, "project-context").with_mime_type("text/plain"),
            Resource::new(MANUSCRIPT_RESOURCE, "manuscript").with_mime_type("text/x-tex"),
            Resource::new(SOURCES_RESOURCE, "sources").with_mime_type("text/plain"),
            Resource::new(REPORTS_RESOURCE, "reports").with_mime_type("text/plain"),
        ]
    }

    fn prompts(&self) -> Vec<Prompt> {
        let mut prompts = Vec::new();
        let skills = self.context.root.join("agent/skills");
        if let Ok(entries) = std::fs::read_dir(skills) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("md")
                    && let Some(name) = path.file_name().and_then(|name| name.to_str())
                {
                    prompts.push(Prompt::new(
                        name,
                        Some("Installed sil skill entrypoint"),
                        None,
                    ));
                }
            }
        }
        prompts.sort_by(|left, right| left.name.cmp(&right.name));
        prompts
    }

    fn read_resource_text(&self, uri: &str) -> Result<String, ErrorData> {
        let path = match uri {
            CONTEXT_RESOURCE => self.context.root.join(".sil/config.yaml"),
            MANUSCRIPT_RESOURCE => self.context.root.join("paper_draft.tex"),
            SOURCES_RESOURCE => self.context.root.join("sources"),
            REPORTS_RESOURCE => self.context.root.join(".sil/reviews"),
            _ => return Err(ErrorData::resource_not_found("Unknown sil resource", None)),
        };
        let confined = self
            .context
            .confine_existing(path.as_str())
            .map_err(|error| ErrorData::invalid_params(error, None))?;
        if confined.is_dir() {
            let mut names = Vec::new();
            for entry in std::fs::read_dir(confined.as_std_path())
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?
            {
                let entry =
                    entry.map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
            names.sort();
            Ok(names.join("\n"))
        } else {
            std::fs::read_to_string(confined.as_std_path())
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))
        }
    }
}

impl ServerHandler for SilMcpHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        ServerInfo::new(capabilities)
            .with_server_info(Implementation::new("sil-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Project-confined scientist-in-loop tools, resources, and skill prompts",
            )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send {
        std::future::ready(Ok(ListToolsResult {
            tools: self.tools(),
            next_cursor: None,
            ..Default::default()
        }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, ErrorData>> + Send {
        let name = request.name.to_string();
        let arguments = request.arguments.map(serde_json::Value::Object);
        let validation = validate_action_arguments(&name, arguments.as_ref());
        let project_context = self.context.clone();
        let request_cancel = context.ct.clone();
        let progress_token = context.meta.get_progress_token();
        let peer = context.peer.clone();
        async move {
            if let Some(error) = validation {
                return Ok(to_rmcp_result(error));
            }
            if request_cancel.is_cancelled() {
                return Ok(to_rmcp_result(CallToolResult::error("Tool call cancelled")));
            }
            if let Some(token) = progress_token.as_ref() {
                let _ = peer
                    .notify_progress(ProgressNotificationParam::new(token.clone(), 0.0))
                    .await;
            }
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(300),
                tokio::task::spawn_blocking(move || {
                    call_tool_with_context(&project_context, &name, arguments)
                }),
            )
            .await
            .map_err(|_| ErrorData::internal_error("Tool call timed out", None))?
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            if let Some(token) = progress_token {
                let _ = peer
                    .notify_progress(ProgressNotificationParam::new(token, 1.0))
                    .await;
            }
            Ok(to_rmcp_result(result))
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send {
        std::future::ready(Ok(ListResourcesResult {
            resources: self.resources(),
            next_cursor: None,
            ..Default::default()
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResponse, ErrorData>> + Send {
        let result = self
            .read_resource_text(&request.uri)
            .map(|text| ReadResourceResult::new(vec![ResourceContents::text(text, request.uri)]));
        std::future::ready(result.map(Into::into))
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, ErrorData>> + Send {
        std::future::ready(Ok(ListPromptsResult {
            prompts: self.prompts(),
            next_cursor: None,
            ..Default::default()
        }))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResponse, ErrorData>> + Send {
        let result = self
            .context
            .skill_path(&request.name)
            .map_err(|error| ErrorData::invalid_params(error, None))
            .and_then(|path| {
                std::fs::read_to_string(path.as_std_path())
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))
            })
            .map(|text| GetPromptResult::new(vec![PromptMessage::new_text(Role::User, text)]));
        std::future::ready(result.map(Into::into))
    }
}

fn to_rmcp_result(result: CallToolResult) -> CallToolResponse {
    let content = result
        .content
        .into_iter()
        .map(|item| match item {
            crate::protocol::Content::Text { text } => ContentBlock::text(text),
        })
        .collect();
    let mut output = rmcp::model::CallToolResult::success(content);
    output.is_error = result.is_error;
    output.into()
}

fn action_variants(name: &str) -> Option<serde_json::Value> {
    let variants = match name {
        "sil_sources" => vec![
            ("search", "query"),
            ("get", "source_id"),
            ("fetch", "target"),
            ("parse", "path"),
            ("rank", "query"),
        ],
        "sil_cite" => vec![
            ("suggest", "query"),
            ("ground", "claim"),
            ("upsert", "entry"),
            ("promote", "cite_key"),
        ],
        "sil_draft" => vec![
            ("edit", "content"),
            ("todo", "content"),
            ("structure", "section_id"),
        ],
        "sil_review" => vec![("estimate", "mode"), ("build", "engine")],
        _ => return None,
    };
    Some(serde_json::Value::Array(
        variants
            .into_iter()
            .map(|(action, required)| {
                serde_json::json!({
                    "type": "object",
                    "properties": { "action": { "const": action } },
                    "required": ["action", required]
                })
            })
            .collect(),
    ))
}

fn validate_action_arguments(
    name: &str,
    arguments: Option<&serde_json::Value>,
) -> Option<CallToolResult> {
    let args = arguments.and_then(serde_json::Value::as_object);
    let action = args
        .and_then(|value| value.get("action"))
        .and_then(serde_json::Value::as_str);
    let required = match (name, action) {
        ("sil_sources", Some("search")) => Some("query"),
        ("sil_sources", Some("get")) => Some("source_id"),
        ("sil_sources", Some("fetch")) => Some("target"),
        ("sil_sources", Some("parse")) => Some("path"),
        ("sil_cite", Some("suggest")) => Some("query"),
        ("sil_cite", Some("ground")) => Some("claim"),
        ("sil_cite", Some("upsert")) => Some("entry"),
        ("sil_cite", Some("promote")) => Some("cite_key"),
        _ => None,
    };
    required.and_then(|field| {
        let present = args
            .and_then(|value| value.get(field))
            .is_some_and(|value| !value.is_null());
        (!present).then(|| CallToolResult::error(format!("Missing required parameter: {field}")))
    })
}

/// Run the official SDK service over stdio.
pub async fn run_stdio(context: McpContext) -> anyhow::Result<()> {
    use rmcp::ServiceExt;
    let (stdin, stdout) = rmcp::transport::io::stdio();
    SilMcpHandler::new(context)
        .serve((stdin, stdout))
        .await?
        .waiting()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::ServiceExt;
    use std::fs;

    fn context() -> (tempfile::TempDir, McpContext) {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        fs::create_dir_all(root.join(".sil/reviews")).unwrap();
        fs::create_dir_all(root.join("agent/skills")).unwrap();
        fs::create_dir_all(root.join("sources")).unwrap();
        fs::write(root.join(".sil/config.yaml"), "version: 1\n").unwrap();
        fs::write(root.join("agent/skills/SYSTEM.md"), "system skill").unwrap();
        fs::write(
            root.join("paper_draft.tex"),
            include_str!("../../../tests/fixtures/pr-v/paper_draft.tex"),
        )
        .unwrap();
        (dir, McpContext::from_root(root).unwrap())
    }

    #[test]
    fn official_handler_advertises_conformance_surface() {
        let (_dir, context) = context();
        let handler = SilMcpHandler::new(context);
        let info = handler.get_info();
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.prompts.is_some());
        let names: Vec<_> = handler.tools().into_iter().map(|tool| tool.name).collect();
        assert_eq!(
            names,
            vec![
                "sil_context",
                "sil_sources",
                "sil_cite",
                "sil_draft",
                "sil_review",
                "sil_propose"
            ]
        );
        let sources = handler
            .tools()
            .into_iter()
            .find(|tool| tool.name == "sil_sources")
            .unwrap();
        assert!(sources.input_schema.get("oneOf").is_some());
    }

    #[test]
    fn resources_and_prompts_are_confined_and_offline() {
        let (_dir, context) = context();
        let handler = SilMcpHandler::new(context);
        assert!(
            handler
                .read_resource_text(MANUSCRIPT_RESOURCE)
                .unwrap()
                .contains("Offline fixture")
        );
        assert!(handler.read_resource_text("file:///etc/passwd").is_err());
        assert_eq!(handler.prompts()[0].name, "SYSTEM.md");
        assert!(handler.context.skill_path("../SYSTEM.md").is_err());
    }

    #[test]
    fn action_validation_is_typed_at_the_protocol_boundary() {
        let error = validate_action_arguments(
            "sil_cite",
            Some(&serde_json::json!({
                "action": "ground"
            })),
        )
        .unwrap();
        assert_eq!(error.is_error, Some(true));
        assert!(
            matches!(&error.content[0], crate::protocol::Content::Text { text } if text.contains("claim"))
        );
        assert!(
            validate_action_arguments(
                "sil_cite",
                Some(&serde_json::json!({
                    "action": "ground", "claim": "A claim"
                }))
            )
            .is_none()
        );
    }

    #[tokio::test]
    async fn official_duplex_transport_negotiates_and_lists_tools() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (_dir, context) = context();
        let (client, server_transport) = tokio::io::duplex(8192);
        let server_task = tokio::spawn(async move {
            SilMcpHandler::new(context)
                .serve(server_transport)
                .await
                .unwrap()
                .waiting()
                .await
                .unwrap();
        });
        let (read, mut write) = tokio::io::split(client);
        let mut reader = BufReader::new(read);
        write.write_all(br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"offline-test","version":"1"}}}"#).await.unwrap();
        write.write_all(b"\n").await.unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let initialize: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(initialize["result"]["serverInfo"]["name"], "sil-mcp");
        write
            .write_all(
                br#"{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
"#,
            )
            .await
            .unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        let tools: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 6);
        drop(write);
        drop(reader);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), server_task).await;
    }
}
