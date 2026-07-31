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

    #[test]
    fn test_handle_fetch_source_missing_target() {
        let res = handle_fetch_source(json!({}));
        assert_eq!(res.is_error, Some(true));
    }
}

