//! Implementation and registration of the 12 core `sil` MCP tools.

use std::fs;
use camino::Utf8PathBuf;
use serde_json::json;
use sil_agent::{ContextFlags, ContextInput, SkillSelection, generate_context, load_skill, sources_summary};
use sil_core::{
    IdeaBlock, ProjectPaths, SciAction, SectionCompletion, Structure,
    project_root_from_cwd, suggest_from_query, suggest_from_source,
};
use sil_db::SilDb;
use sil_git::{proposal_for_action, propose_from_status, status};
use sil_latex::{audit_manuscript, build_command, parse_idea_blocks, update_or_insert_idea_block};

use crate::protocol::{CallToolResult, Tool, ToolInputSchema};

/// Returns all 12 core `sil` registered tools with valid JSON schemas.
pub fn list_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "sil_search_sources".to_string(),
            description: "Hybrid RAG search (BM25 + Local ONNX Dense RRF + HyDE + Parent expansion)".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "query": { "type": "string", "description": "Search query string" },
                    "limit": { "type": "integer", "description": "Max search results (default 5)" },
                    "hyde": { "type": "boolean", "description": "Use HyDE expansion (default false)" },
                    "expand_parent": { "type": "boolean", "description": "Expand matching chunks to full parent section context (default true)" }
                }),
                vec!["query"],
            ),
        },
        Tool {
            name: "sil_get_source_context".to_string(),
            description: "Fetch full parent chunk/section context for a source".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "source_id": { "type": "string", "description": "Source ID" },
                    "chunk_id": { "type": "string", "description": "Optional specific chunk ID" }
                }),
                vec!["source_id"],
            ),
        },
        Tool {
            name: "sil_suggest_citations".to_string(),
            description: "Suggest deterministic BibTeX & \\cite{} keys".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "query": { "type": "string", "description": "Query or paper title" },
                    "source_id": { "type": "string", "description": "Optional source ID" }
                }),
                vec!["query"],
            ),
        },
        Tool {
            name: "sil_list_todos".to_string(),
            description: "Query TODOs with status, priority, section filters".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "status": { "type": "string", "description": "Filter by status: open, in_progress, done" },
                    "priority": { "type": "string", "description": "Filter by priority: high, medium, low" },
                    "section": { "type": "string", "description": "Filter by section ID" },
                    "sort_by": { "type": "string", "description": "Sort order: priority or line_start" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_update_todo".to_string(),
            description: "Create/update # -- X -- # block in paper_draft.tex & DB".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "id": { "type": "string", "description": "Optional existing block ID to update" },
                    "section_id": { "type": "string", "description": "Section ID" },
                    "content": { "type": "string", "description": "Content of the TODO/idea block" },
                    "status": { "type": "string", "description": "Status: open, in_progress, done" },
                    "priority": { "type": "string", "description": "Priority: high, medium, low" }
                }),
                vec!["content"],
            ),
        },
        Tool {
            name: "sil_list_skills".to_string(),
            description: "Discover local & environmental skills".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "category": { "type": "string", "description": "Optional category filter" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_invoke_skill".to_string(),
            description: "Programmatically execute a skill".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "name": { "type": "string", "description": "Name of skill (e.g. paper.md, SYSTEM.md)" },
                    "input": { "type": "string", "description": "Optional skill input" }
                }),
                vec!["name"],
            ),
        },
        Tool {
            name: "sil_get_workspace_context".to_string(),
            description: "Synthesized sil context snapshot".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "include_sources": { "type": "boolean", "description": "Include sources summary (default true)" },
                    "include_paper": { "type": "boolean", "description": "Include paper draft (default true)" },
                    "include_todos": { "type": "boolean", "description": "Include active TODOs (default true)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_get_structure".to_string(),
            description: "Read/update completion in structure.yaml".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "action": { "type": "string", "description": "'read' or 'update'" },
                    "section_id": { "type": "string", "description": "Section ID to update" },
                    "completed": { "type": "boolean", "description": "Completion status" },
                    "word_count": { "type": "integer", "description": "Optional target word count" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_build_and_doctor".to_string(),
            description: "Compile paper & health check (sil build, sil doctor)".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "engine": { "type": "string", "description": "LaTeX engine (pdflatex, xelatex, lualatex, tectonic)" },
                    "run_doctor": { "type": "boolean", "description": "Run health audit (default true)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_propose_commit".to_string(),
            description: "Generate Sci-Action commit proposal (NEVER auto-commits)".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "message": { "type": "string", "description": "Optional custom commit message" },
                    "action": { "type": "string", "description": "Sci-Action category" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_fetch_source".to_string(),
            description: "Download paper/source into sources/ by DOI, arXiv ID, or URL and optionally parse into SQLite".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "target": { "type": "string", "description": "DOI (10.xxxx), arXiv ID (arxiv:XXXX.YYYY), or direct URL" },
                    "no_parse": { "type": "boolean", "description": "Skip immediate parsing after download (default false)" }
                }),
                vec!["target"],
            ),
        },
    ]
}

/// Execute a tool call by name with given parameters.
pub fn call_tool(name: &str, arguments: Option<serde_json::Value>) -> CallToolResult {
    let args = arguments.unwrap_or_else(|| json!({}));

    match name {
        "sil_search_sources" => handle_search_sources(args),
        "sil_get_source_context" => handle_get_source_context(args),
        "sil_suggest_citations" => handle_suggest_citations(args),
        "sil_list_todos" => handle_list_todos(args),
        "sil_update_todo" => handle_update_todo(args),
        "sil_list_skills" => handle_list_skills(args),
        "sil_invoke_skill" => handle_invoke_skill(args),
        "sil_get_workspace_context" => handle_get_workspace_context(args),
        "sil_get_structure" => handle_get_structure(args),
        "sil_build_and_doctor" => handle_build_and_doctor(args),
        "sil_propose_commit" => handle_propose_commit(args),
        "sil_fetch_source" => handle_fetch_source(args),
        _ => CallToolResult::error(format!("Unknown tool: {name}")),
    }
}

fn get_project_paths() -> Result<(Utf8PathBuf, ProjectPaths), String> {
    let root = project_root_from_cwd().map_err(|e| format!("Not in a sil project: {e}"))?;
    let paths = ProjectPaths::new(&root);
    Ok((root, paths))
}

fn handle_search_sources(args: serde_json::Value) -> CallToolResult {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return CallToolResult::error("Missing required parameter: query"),
    };
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let hyde = args.get("hyde").and_then(|v| v.as_bool()).unwrap_or(false);
    let expand_parent = args.get("expand_parent").and_then(|v| v.as_bool()).unwrap_or(true);

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let db = match SilDb::open(&paths.db()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::error(format!("Failed to open database: {e}")),
    };

    let global_settings = sil_core::GlobalSettings::load_or_default(None);
    let rag = get_project_paths()
        .ok()
        .and_then(|(_r, p)| sil_core::Config::load(&p.config()).ok())
        .and_then(|cfg| cfg.rag)
        .unwrap_or(global_settings.rag);

    let embedder = Some(sil_db::OnnxEmbedder::from_rag_settings(&rag));

    if hyde {
        if let Some(ref emb) = embedder {
            match db.search_hyde(emb, query, query, limit, expand_parent) {
                Ok(hits) => {
                    let res: Vec<_> = hits.into_iter().map(|h| format_chunk_hit(&h)).collect();
                    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
                }
                Err(e) => CallToolResult::error(format!("HyDE search error: {e}")),
            }
        } else {
            match db.search(query, limit) {
                Ok(hits) => {
                    let res: Vec<_> = hits.into_iter().map(|h| json!({
                        "source_id": h.id.as_str(),
                        "filename": h.filename,
                        "title": h.title,
                        "snippet": h.snippet,
                    })).collect();
                    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
                }
                Err(e) => CallToolResult::error(format!("FTS search error: {e}")),
            }
        }
    } else if let Some(ref emb) = embedder {
        match db.search_hybrid(emb, query, limit, expand_parent) {
            Ok(hits) => {
                let res: Vec<_> = hits.into_iter().map(|h| format_chunk_hit(&h)).collect();
                CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
            }
            Err(e) => CallToolResult::error(format!("Hybrid search error: {e}")),
        }
    } else {
        match db.search(query, limit) {
            Ok(hits) => {
                let res: Vec<_> = hits.into_iter().map(|h| json!({
                    "source_id": h.id.as_str(),
                    "filename": h.filename,
                    "title": h.title,
                    "snippet": h.snippet,
                })).collect();
                CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
            }
            Err(e) => CallToolResult::error(format!("FTS search error: {e}")),
        }
    }
}

fn format_chunk_hit(hit: &sil_db::ChunkSearchHit) -> serde_json::Value {
    json!({
        "score": hit.score,
        "chunk_id": hit.chunk.id,
        "source_id": hit.chunk.source_id.as_str(),
        "chunk_type": hit.chunk.chunk_type.as_str(),
        "heading_title": hit.chunk.heading_title,
        "content": hit.chunk.content,
        "snippet": hit.snippet,
    })
}

fn handle_get_source_context(args: serde_json::Value) -> CallToolResult {
    let source_id = match args.get("source_id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return CallToolResult::error("Missing required parameter: source_id"),
    };
    let chunk_id = args.get("chunk_id").and_then(|v| v.as_str());

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let db = match SilDb::open(&paths.db()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::error(format!("Failed to open database: {e}")),
    };

    if let Some(cid) = chunk_id {
        match db.get_chunk_by_id(cid) {
            Ok(Some(chunk)) => {
                let parent_chunk = if let Some(ref pid) = chunk.parent_chunk_id {
                    db.get_chunk_by_id(pid).ok().flatten()
                } else {
                    None
                };
                let res = json!({
                    "chunk": {
                        "id": chunk.id,
                        "source_id": chunk.source_id.as_str(),
                        "heading_title": chunk.heading_title,
                        "chunk_type": chunk.chunk_type.as_str(),
                        "content": chunk.content,
                    },
                    "parent_context": parent_chunk.map(|p| json!({
                        "id": p.id,
                        "heading_title": p.heading_title,
                        "content": p.content,
                    }))
                });
                CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
            }
            Ok(None) => CallToolResult::error(format!("Chunk '{cid}' not found")),
            Err(e) => CallToolResult::error(format!("Failed to query chunk: {e}")),
        }
    } else {
        let sid = sil_core::SourceId::new(source_id);
        match db.get_chunks_for_source(&sid) {
            Ok(chunks) => {
                let chunks_json: Vec<_> = chunks.iter().map(|c| json!({
                    "id": c.id,
                    "chunk_type": c.chunk_type.as_str(),
                    "heading_title": c.heading_title,
                    "content": c.content,
                })).collect();
                let res = json!({
                    "source_id": source_id,
                    "chunk_count": chunks.len(),
                    "chunks": chunks_json
                });
                CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
            }
            Err(e) => CallToolResult::error(format!("Failed to query source chunks: {e}")),
        }
    }
}

fn handle_suggest_citations(args: serde_json::Value) -> CallToolResult {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return CallToolResult::error("Missing required parameter: query"),
    };
    let source_id = args.get("source_id").and_then(|v| v.as_str());

    if let Some(sid) = source_id {
        let doc_opt = get_project_paths()
            .ok()
            .and_then(|(_root, paths)| SilDb::open(&paths.db()).ok())
            .and_then(|db| db.list_sources().ok())
            .and_then(|sources| sources.into_iter().find(|s| s.id.as_str() == sid));

        if let Some(doc) = doc_opt {
            let suggestion = suggest_from_source(&doc);
            return CallToolResult::text(serde_json::to_string_pretty(&json!({
                "bibtex": suggestion.bibtex,
                "cite_command": suggestion.cite_command,
                "key": suggestion.cite_key,
                "note": suggestion.note
            })).unwrap_or_default());
        }
    }

    let suggestion = suggest_from_query(query);
    let res = json!({
        "bibtex": suggestion.bibtex,
        "cite_command": suggestion.cite_command,
        "key": suggestion.cite_key,
        "note": suggestion.note
    });
    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_list_todos(args: serde_json::Value) -> CallToolResult {
    let status = args.get("status").and_then(|v| v.as_str());
    let priority = args.get("priority").and_then(|v| v.as_str());
    let section = args.get("section").and_then(|v| v.as_str());
    let sort_by = args.get("sort_by").and_then(|v| v.as_str());

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let draft_path = paths.paper_draft();
    if let Ok(tex) = fs::read_to_string(draft_path.as_str()) {
        let ideas = parse_idea_blocks(&tex);
        if let Ok(db) = SilDb::open(&paths.db()) {
            let _ = db.replace_todo_ideas(&ideas);
        }
    }

    let db = match SilDb::open(&paths.db()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::error(format!("Failed to open DB: {e}")),
    };

    match db.list_todo_ideas_filtered(status, priority, section, sort_by) {
        Ok(todos) => CallToolResult::text(serde_json::to_string_pretty(&todos).unwrap_or_default()),
        Err(e) => CallToolResult::error(format!("Failed to query TODOs: {e}")),
    }
}

fn handle_update_todo(args: serde_json::Value) -> CallToolResult {
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return CallToolResult::error("Missing required parameter: content"),
    };
    let id = args.get("id").and_then(|v| v.as_str());
    let section_id = args.get("section_id").and_then(|v| v.as_str());
    let status = args.get("status").and_then(|v| v.as_str());
    let priority = args.get("priority").and_then(|v| v.as_str());

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let draft_path = paths.paper_draft();
    let tex = if draft_path.exists() {
        fs::read_to_string(draft_path.as_str()).unwrap_or_default()
    } else {
        String::new()
    };

    let mut block = IdeaBlock::new(id.unwrap_or(""), content, section_id.map(String::from), 0, 0);
    if let Some(st) = status {
        block.status = st.to_string();
    }
    if let Some(pr) = priority {
        block.priority = pr.to_string();
    }

    let updated_tex = update_or_insert_idea_block(&tex, &block);
    if let Err(e) = fs::write(draft_path.as_str(), &updated_tex) {
        return CallToolResult::error(format!("Failed to write paper_draft.tex: {e}"));
    }

    let ideas = parse_idea_blocks(&updated_tex);
    if let Ok(db) = SilDb::open(&paths.db()) {
        let _ = db.replace_todo_ideas(&ideas);
    }

    CallToolResult::text(json!({
        "status": "updated",
        "content": content,
        "section_id": section_id,
        "idea_blocks_count": ideas.len()
    }).to_string())
}

fn handle_list_skills(args: serde_json::Value) -> CallToolResult {
    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let skills_dir = paths.skills_dir();
    let mut skills = vec![
        json!({ "name": "SYSTEM.md", "type": "built-in", "description": "Core system instructions and agent rules" }),
        json!({ "name": "paper.md", "type": "built-in", "description": "Scientific paper drafting guidance" }),
        json!({ "name": "agent-code.md", "type": "built-in", "description": "Code generation and architecture rules" }),
    ];

    if let Ok(entries) = fs::read_dir(skills_dir.as_str()) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".md") && !skills.iter().any(|s| s["name"] == name) {
                    skills.push(json!({
                        "name": name,
                        "type": "custom",
                        "path": p.to_string_lossy()
                    }));
                }
            }
        }
    }

    let category = args.get("category").and_then(|v| v.as_str());
    if let Some(cat) = category {
        skills.retain(|s| s["type"] == cat || s["name"].as_str().unwrap_or("").contains(cat));
    }

    CallToolResult::text(serde_json::to_string_pretty(&skills).unwrap_or_default())
}

fn handle_invoke_skill(args: serde_json::Value) -> CallToolResult {
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return CallToolResult::error("Missing required parameter: name"),
    };
    let input = args.get("input").and_then(|v| v.as_str());

    let (root, _paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    match load_skill(&root, name) {
        Ok(content) => {
            let res = json!({
                "skill": name,
                "input": input,
                "content": content
            });
            CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        Err(e) => CallToolResult::error(format!("Failed to load skill '{name}': {e}")),
    }
}

fn handle_get_workspace_context(args: serde_json::Value) -> CallToolResult {
    let include_paper = args.get("include_paper").and_then(|v| v.as_bool()).unwrap_or(true);

    let (root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let config_yaml = fs::read_to_string(paths.config().as_str()).unwrap_or_default();
    let structure_yaml = fs::read_to_string(paths.structure().as_str()).unwrap_or_default();
    let structure = Structure::load(&paths.structure()).ok();
    let db = SilDb::open(&paths.db()).ok();
    let summary = db.as_ref().and_then(|d| sources_summary(d).ok()).unwrap_or_default();
    let log = sil_git::log_entries(&root, 10, true).unwrap_or_default();

    let flags = ContextFlags {
        paper: include_paper,
        agent: false,
        skill_paper: true,
        skill_agent_code: false,
        skills: vec![],
    };
    let skills = SkillSelection::always();

    let input = ContextInput {
        root: &root,
        config_yaml: &config_yaml,
        structure_yaml: &structure_yaml,
        structure: structure.as_ref(),
        sources_summary: &summary,
        log_entries: &log,
        flags: &flags,
        skills,
    };

    match generate_context(&input) {
        Ok(ctx) => CallToolResult::text(ctx),
        Err(e) => CallToolResult::error(format!("Failed to generate context: {e}")),
    }
}

fn handle_get_structure(args: serde_json::Value) -> CallToolResult {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("read");
    let section_id = args.get("section_id").and_then(|v| v.as_str());
    let completed = args.get("completed").and_then(|v| v.as_bool());

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let mut struct_obj = match Structure::load(&paths.structure()) {
        Ok(s) => s,
        Err(e) => return CallToolResult::error(format!("Failed to load structure.yaml: {e}")),
    };

    if action == "update" {
        let sid = match section_id {
            Some(s) => s,
            None => return CallToolResult::error("Missing section_id for update action"),
        };
        let is_comp = match completed {
            Some(c) => c,
            None => return CallToolResult::error("Missing completed status for update action"),
        };

        if let Some(sec) = struct_obj.sections.iter_mut().find(|s| s.id == sid) {
            sec.completion = if is_comp {
                SectionCompletion::Draft
            } else {
                SectionCompletion::Empty
            };
            if let Err(e) = struct_obj.save(&paths.structure()) {
                return CallToolResult::error(format!("Failed to save structure.yaml: {e}"));
            }
        } else {
            return CallToolResult::error(format!("Section '{sid}' not found in structure.yaml"));
        }
    }

    let summary = struct_obj.completion_summary();
    let res = json!({
        "structure": struct_obj,
        "completion_summary": {
            "total": summary.total,
            "empty": summary.empty,
            "outline": summary.outline,
            "draft": summary.draft,
            "polished": summary.polished,
            "summary": summary.to_string(),
        },
    });
    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_build_and_doctor(args: serde_json::Value) -> CallToolResult {
    let engine_str = args.get("engine").and_then(|v| v.as_str()).unwrap_or("pdflatex");
    let run_doctor = args.get("run_doctor").and_then(|v| v.as_bool()).unwrap_or(true);

    let (root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let engine = match engine_str {
        "xelatex" => sil_core::LatexEngine::Xelatex,
        "lualatex" => sil_core::LatexEngine::Lualatex,
        "tectonic" => sil_core::LatexEngine::Tectonic,
        _ => sil_core::LatexEngine::Pdflatex,
    };

    let cmd = build_command(engine, &paths.paper_draft(), &root);

    let mut report = None;
    if run_doctor {
        report = audit_manuscript(&paths.paper_draft(), None)
            .ok()
            .map(|r| serde_json::to_value(&r).unwrap_or_default());
    }

    let res = json!({
        "build_command": format!("{cmd:?}"),
        "engine": engine_str,
        "health_doctor_report": report
    });

    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_propose_commit(args: serde_json::Value) -> CallToolResult {
    let message = args.get("message").and_then(|v| v.as_str());
    let action_str = args.get("action").and_then(|v| v.as_str());

    let (root, _paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let git_status = match status(&root) {
        Ok(s) => s,
        Err(e) => return CallToolResult::error(format!("Failed to get git status: {e}")),
    };

    let explicit_action = action_str.and_then(|a| a.parse::<SciAction>().ok());

    let proposal = match propose_from_status(&git_status, explicit_action, message, None) {
        Ok(p) => p,
        Err(_) => {
            let act = explicit_action.unwrap_or(SciAction::EditDraft);
            proposal_for_action(act, message, None)
        }
    };

    let res = json!({
        "proposal_subject": proposal.subject,
        "proposal_body": proposal.body.join("\n"),
        "full_commit_message": proposal.message(),
        "action_trailer": proposal.action.as_str(),
        "warning": "This proposal is generated for review and will NEVER be committed automatically."
    });

    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_fetch_source(args: serde_json::Value) -> CallToolResult {
    let target = match args.get("target").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return CallToolResult::error("Missing required parameter: target"),
    };
    let no_parse = args.get("no_parse").and_then(|v| v.as_bool()).unwrap_or(false);

    let (root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let config = sil_core::Config::load(&paths.config()).unwrap_or_default();
    let sources_dir = paths.sources(&config);

    let db = match SilDb::open(&paths.db()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::error(format!("Failed to open database: {e}")),
    };

    let saved_path = match sil_parse::fetch_source_target(target, &sources_dir) {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(format!("Fetch failed: {e}")),
    };

    let downloaded_path_str = saved_path.as_str().to_string();

    let pdf_path = if saved_path.is_absolute() {
        saved_path.clone()
    } else if sources_dir
        .join(saved_path.file_name().unwrap_or(saved_path.as_str()))
        .exists()
    {
        sources_dir.join(saved_path.file_name().unwrap_or(saved_path.as_str()))
    } else {
        root.join(&saved_path)
    };

    let mut parsed = false;
    let mut title: Option<String> = None;
    let mut source_id: Option<String> = None;

    if !no_parse && pdf_path.exists() {
        let null_ui = sil_core::NullUi::new();
        let runner_res: Result<Box<dyn sil_parse::MarkerRunner>, sil_parse::ParseError> =
            if let Ok(stub) = std::env::var("SIL_MARKER_STUB") {
                Ok(Box::new(sil_parse::StubMarkerRunner { content: stub }))
            } else {
                sil_parse::PythonMarkerRunner::discover()
                    .map(|r| Box::new(r) as Box<dyn sil_parse::MarkerRunner>)
            };

        if let Ok(runner) = runner_res
            && let Ok(res) = sil_parse::parse_one(&pdf_path, &db, runner.as_ref(), &null_ui)
        {
            parsed = res.document.parsed;
            title = res.document.title;
            source_id = Some(res.document.id.as_str().to_string());
        }
    }

    let proposal = proposal_for_action(
        SciAction::FetchSource,
        Some(&format!("Fetch source: {target}")),
        Some(&format!("Saved to {downloaded_path_str}")),
    );

    let res = json!({
        "downloaded_path": downloaded_path_str,
        "parsed": parsed,
        "title": title,
        "source_id": source_id,
        "commit_proposal": {
            "proposal_subject": proposal.subject,
            "proposal_body": proposal.body.join("\n"),
            "full_commit_message": proposal.message(),
            "action_trailer": proposal.action.as_str(),
        }
    });

    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Content;
    use std::fs;
    use std::sync::{Mutex, MutexGuard};
    use tempfile::TempDir;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    struct TestEnv {
        _guard: MutexGuard<'static, ()>,
        _dir: TempDir,
        orig_cwd: std::path::PathBuf,
        project_root: Utf8PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            let orig_cwd = std::env::current_dir().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let project_root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

            fs::create_dir_all(project_root.join(".sil")).unwrap();
            fs::write(project_root.join(".sil/config.yaml"), "version: 1\n").unwrap();

            std::env::set_current_dir(&project_root).unwrap();

            Self {
                _guard: guard,
                _dir: dir,
                orig_cwd,
                project_root,
            }
        }

        fn setup_db(&self) -> SilDb {
            let paths = ProjectPaths::new(&self.project_root);
            SilDb::open(&paths.db()).unwrap()
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.orig_cwd);
        }
    }

    fn extract_text(res: &CallToolResult) -> &str {
        match &res.content[0] {
            Content::Text { text } => text.as_str(),
        }
    }

    #[test]
    fn test_handle_fetch_source_missing_target() {
        let res = handle_fetch_source(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: target"));
    }

    #[test]
    fn test_handle_search_sources() {
        // Missing query validation
        let res_missing = handle_search_sources(json!({}));
        assert_eq!(res_missing.is_error, Some(true));
        assert!(extract_text(&res_missing).contains("Missing required parameter: query"));

        // Execution on temp project DB
        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("paper_fts");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/paper_fts.pdf"));
        doc.id = sid.clone();
        doc.title = Some("Quantum Dynamics".to_string());
        db.upsert_parsed(&doc, "Quantum dynamics and state vector evolution.").unwrap();

        let chunk = sil_db::SourceChunk {
            id: "chk_fts_1".to_string(),
            source_id: sid,
            parent_chunk_id: None,
            chunk_type: sil_db::ChunkType::Parent,
            heading_title: Some("Quantum Section".to_string()),
            content: "Quantum dynamics and state vector evolution.".to_string(),
            start_offset: 0,
            end_offset: 40,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_source_chunks(&[chunk]).unwrap();

        let res = handle_search_sources(json!({
            "query": "Quantum",
            "limit": 5,
            "hyde": false
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        assert!(extract_text(&res).contains("paper_fts"));
    }

    #[test]
    fn test_handle_get_source_context() {
        // Missing source_id validation
        let res_missing = handle_get_source_context(json!({}));
        assert_eq!(res_missing.is_error, Some(true));
        assert!(extract_text(&res_missing).contains("Missing required parameter: source_id"));

        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("src_ctx_test");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/src_ctx_test.pdf"));
        doc.id = sid.clone();
        db.upsert_parsed(&doc, "source text").unwrap();

        let parent_chunk = sil_db::SourceChunk {
            id: "p1".to_string(),
            source_id: sid.clone(),
            parent_chunk_id: None,
            chunk_type: sil_db::ChunkType::Parent,
            heading_title: Some("Introduction".to_string()),
            content: "Parent section content".to_string(),
            start_offset: 0,
            end_offset: 20,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let child_chunk = sil_db::SourceChunk {
            id: "c1".to_string(),
            source_id: sid,
            parent_chunk_id: Some("p1".to_string()),
            chunk_type: sil_db::ChunkType::Child,
            heading_title: None,
            content: "Child paragraph text".to_string(),
            start_offset: 21,
            end_offset: 40,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_source_chunks(&[parent_chunk, child_chunk]).unwrap();

        // Query by source_id without chunk_id
        let res_source = handle_get_source_context(json!({ "source_id": "src_ctx_test" }));
        assert!(res_source.is_error.is_none() || res_source.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res_source)).unwrap();
        assert_eq!(val["source_id"], "src_ctx_test");
        assert_eq!(val["chunk_count"], 2);

        // Query by chunk_id
        let res_chunk = handle_get_source_context(json!({
            "source_id": "src_ctx_test",
            "chunk_id": "c1"
        }));
        assert!(res_chunk.is_error.is_none() || res_chunk.is_error == Some(false));
        let val_c: serde_json::Value = serde_json::from_str(extract_text(&res_chunk)).unwrap();
        assert_eq!(val_c["chunk"]["id"], "c1");
        assert_eq!(val_c["parent_context"]["id"], "p1");

        // Query missing chunk_id
        let res_missing_chunk = handle_get_source_context(json!({
            "source_id": "src_ctx_test",
            "chunk_id": "nonexistent"
        }));
        assert_eq!(res_missing_chunk.is_error, Some(true));
        assert!(extract_text(&res_missing_chunk).contains("Chunk 'nonexistent' not found"));
    }

    #[test]
    fn test_handle_list_and_update_todos() {
        let env = TestEnv::new();

        let initial_tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=todo_1, priority=high, status=open]
% Add background
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), initial_tex).unwrap();

        // 1. List TODOs
        let res_list = handle_list_todos(json!({ "status": "open", "priority": "high" }));
        assert!(res_list.is_error.is_none() || res_list.is_error == Some(false));
        assert!(extract_text(&res_list).contains("Add background"));

        // 2. Update TODO missing content validation
        let res_err = handle_update_todo(json!({}));
        assert_eq!(res_err.is_error, Some(true));
        assert!(extract_text(&res_err).contains("Missing required parameter: content"));

        // 3. Update TODO
        let res_upd = handle_update_todo(json!({
            "id": "todo_1",
            "section_id": "sec_bg",
            "content": "Add complete background survey",
            "status": "done",
            "priority": "low"
        }));
        assert!(res_upd.is_error.is_none() || res_upd.is_error == Some(false));
        assert!(extract_text(&res_upd).contains("\"status\":\"updated\""));

        let updated_tex = fs::read_to_string(env.project_root.join("paper_draft.tex")).unwrap();
        assert!(updated_tex.contains("Add complete background survey"));
        assert!(updated_tex.contains("status=done"));
        assert!(updated_tex.contains("priority=low"));
    }

    #[test]
    fn test_handle_list_and_invoke_skills() {
        let env = TestEnv::new();

        let skills_dir = env.project_root.join(".sil/skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("custom-tool.md"), "# Custom Tool\nInstructions.").unwrap();

        // 1. List skills
        let res_list = handle_list_skills(json!({}));
        assert!(res_list.is_error.is_none() || res_list.is_error == Some(false));
        let list_json: serde_json::Value = serde_json::from_str(extract_text(&res_list)).unwrap();
        assert!(list_json.as_array().unwrap().iter().any(|s| s["name"] == "SYSTEM.md"));
        assert!(list_json.as_array().unwrap().iter().any(|s| s["name"] == "custom-tool.md"));

        // 2. Filter by category
        let res_cat = handle_list_skills(json!({ "category": "custom" }));
        let cat_json: serde_json::Value = serde_json::from_str(extract_text(&res_cat)).unwrap();
        assert!(cat_json.as_array().unwrap().iter().all(|s| s["type"] == "custom" || s["name"].as_str().unwrap_or("").contains("custom")));

        // 3. Invoke missing name parameter
        let res_no_name = handle_invoke_skill(json!({}));
        assert_eq!(res_no_name.is_error, Some(true));
        assert!(extract_text(&res_no_name).contains("Missing required parameter: name"));

        // 4. Invoke custom skill
        let res_inv = handle_invoke_skill(json!({
            "name": "custom-tool.md",
            "input": "test input data"
        }));
        assert!(res_inv.is_error.is_none() || res_inv.is_error == Some(false));
        let inv_val: serde_json::Value = serde_json::from_str(extract_text(&res_inv)).unwrap();
        assert_eq!(inv_val["skill"], "custom-tool.md");
        assert_eq!(inv_val["input"], "test input data");
        assert!(inv_val["content"].as_str().unwrap().contains("Custom Tool"));
    }

    #[test]
    fn test_handle_get_structure() {
        let env = TestEnv::new();

        let struct_yaml = r#"
title: "Test Paper"
status: "draft"
sections:
  - id: "sec_1"
    title: "Intro"
    level: 1
    completion: "empty"
"#;
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        // 1. Read structure
        let res_read = handle_get_structure(json!({ "action": "read" }));
        assert!(res_read.is_error.is_none() || res_read.is_error == Some(false));
        let val_read: serde_json::Value = serde_json::from_str(extract_text(&res_read)).unwrap();
        assert_eq!(val_read["structure"]["title"], "Test Paper");
        assert_eq!(val_read["completion_summary"]["total"], 1);

        // 2. Update section completion
        let res_upd = handle_get_structure(json!({
            "action": "update",
            "section_id": "sec_1",
            "completed": true
        }));
        assert!(res_upd.is_error.is_none() || res_upd.is_error == Some(false));
        let val_upd: serde_json::Value = serde_json::from_str(extract_text(&res_upd)).unwrap();
        assert_eq!(val_upd["completion_summary"]["draft"], 1);

        // 3. Update missing section_id -> error
        let res_no_sid = handle_get_structure(json!({ "action": "update", "completed": true }));
        assert_eq!(res_no_sid.is_error, Some(true));

        // 4. Update unknown section_id -> error
        let res_unk_sid = handle_get_structure(json!({ "action": "update", "section_id": "unknown_sec", "completed": true }));
        assert_eq!(res_unk_sid.is_error, Some(true));
        assert!(extract_text(&res_unk_sid).contains("Section 'unknown_sec' not found"));
    }

    #[test]
    fn test_handle_build_and_doctor() {
        let env = TestEnv::new();
        fs::write(env.project_root.join("paper_draft.tex"), "\\documentclass{article}\n\\begin{document}\nTest\n\\end{document}").unwrap();

        for engine in &["pdflatex", "xelatex", "lualatex", "tectonic"] {
            let res = handle_build_and_doctor(json!({
                "engine": engine,
                "run_doctor": true
            }));
            assert!(res.is_error.is_none() || res.is_error == Some(false));
            let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
            assert_eq!(val["engine"], *engine);
            assert!(val["build_command"].as_str().unwrap().contains(engine));
            assert!(val["health_doctor_report"].is_object());
        }
    }

    #[test]
    fn test_handle_propose_commit() {
        let env = TestEnv::new();

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&env.project_root)
            .output()
            .unwrap();

        let res = handle_propose_commit(json!({
            "message": "Add intro section",
            "action": "edit-draft"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["action_trailer"], "edit-draft");
        assert!(val["warning"].as_str().unwrap().contains("NEVER be committed automatically"));
    }

    #[test]
    fn test_handle_fetch_source_with_mock_script() {
        let env = TestEnv::new();

        let mock_script_path = env.project_root.join("mock_download.py");
        fs::write(
            &mock_script_path,
            "import sys\nprint('sources/mock_paper.pdf')\n",
        )
        .unwrap();

        let sources_dir = env.project_root.join("sources");
        fs::create_dir_all(&sources_dir).unwrap();
        fs::write(sources_dir.join("mock_paper.pdf"), "%PDF-1.4 dummy pdf content").unwrap();

        unsafe {
            std::env::set_var("SIL_DOWNLOAD_SCRIPT", mock_script_path.as_str());
            std::env::set_var("SIL_MARKER_STUB", "Stub text content");
        }

        let res = handle_fetch_source(json!({
            "target": "10.1000/182"
        }));

        unsafe {
            std::env::remove_var("SIL_DOWNLOAD_SCRIPT");
            std::env::remove_var("SIL_MARKER_STUB");
        }

        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["downloaded_path"], "sources/mock_paper.pdf");
        assert!(val["commit_proposal"].is_object());
    }
}


