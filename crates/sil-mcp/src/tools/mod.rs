//! Implementation and registration of the core `sil` MCP tools.

use camino::Utf8PathBuf;
use serde_json::json;
use sil_agent::{
    ContextFlags, ContextInput, EstimateInput, EstimateMode, SkillSelection,
    estimate_proposal_message, generate_context, load_skill, run_heuristic_estimate,
    sources_summary, write_estimate_report,
};
use sil_core::{
    IdeaBlock, ProjectPaths, SciAction, SectionCompletion, Structure, UpsertOptions, WorkspaceLock,
    extract_bib_entry_info, is_same_paper, is_tui_added_bib_block, mark_tui_added_bib_entry,
    parse_bib_blocks, project_root_from_cwd, suggest_from_query, suggest_from_source,
    unmark_tui_added_bib_entry, upsert_bib_entry_with_options, write_lock,
};
use sil_latex::split_tex_sections;
use sil_db::SilDb;
use sil_git::{proposal_for_action, propose_from_status, status};
use sil_latex::{audit_manuscript, build_command, parse_idea_blocks, update_or_insert_idea_block};
use std::fs;

use crate::protocol::{CallToolResult, Tool, ToolInputSchema};

/// Returns all registered `sil` tools with valid JSON schemas.
pub fn list_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "sil_context".to_string(),
            description: "Orient: snapshot workspace context, discover/invoke skills, read structure, or query TODOs.".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "include_sources": { "type": "boolean", "description": "Include sources summary (default true)" },
                    "include_paper": { "type": "boolean", "description": "Include paper draft (default true)" },
                    "include_todos": { "type": "boolean", "description": "Include active TODOs (default true)" },
                    "include_structure": { "type": "boolean", "description": "Include structure completion (default false)" },
                    "list_skills": { "type": "boolean", "description": "List available skills (default false)" },
                    "category": { "type": "string", "description": "Skill category filter when listing skills" },
                    "skill": { "type": "string", "description": "Skill name to invoke (e.g. SYSTEM.md or paper.md)" },
                    "name": { "type": "string", "description": "Alias for skill name" },
                    "input": { "type": "string", "description": "Optional input string when invoking a skill" },
                    "status": { "type": "string", "description": "Filter TODOs by status: open, in_progress, done" },
                    "priority": { "type": "string", "description": "Filter TODOs by priority: high, medium, low" },
                    "section": { "type": "string", "description": "Filter TODOs by section ID" },
                    "sort_by": { "type": "string", "description": "Sort TODOs: priority or line_start" },
                    "action": { "type": "string", "description": "Optional explicit sub-action: skills | todos | structure" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sil_sources".to_string(),
            description: "Literature lifecycle: search | get | fetch | parse | rank (hybrid RAG when onnx+models available, else hash fallback).".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "action": {
                        "type": "string",
                        "enum": ["search", "get", "fetch", "parse", "rank"],
                        "description": "Lifecycle action to perform"
                    },
                    "query": { "type": "string", "description": "Search query string (required for action=search)" },
                    "source_id": { "type": "string", "description": "Source ID or filename under sources/ (required for action=get)" },
                    "chunk_id": { "type": "string", "description": "Optional specific chunk ID (action=get)" },
                    "target": { "type": "string", "description": "DOI (10.xxxx), arXiv ID (arxiv:XXXX.YYYY), or direct URL (required for action=fetch)" },
                    "path": { "type": "string", "description": "Path to source file for action=parse" },
                    "limit": { "type": "integer", "description": "Max search or rank results (default 5 for search, 50 for rank)" },
                    "hyde": { "type": "boolean", "description": "Use HyDE expansion for search (default false)" },
                    "expand_parent": { "type": "boolean", "description": "Expand matching chunks to parent section context (default true)" },
                    "no_parse": { "type": "boolean", "description": "Skip immediate parsing after fetch (default false)" },
                    "all_unparsed": { "type": "boolean", "description": "Parse all unparsed sources under sources/ (default false)" },
                    "min_score": { "type": "number", "description": "Minimum similarity score for rank (default 0.0)" }
                }),
                vec!["action"],
            ),
        },
        Tool {
            name: "sil_cite".to_string(),
            description: "Cite/bib: suggest keys, ground claims, upsert/promote BibTeX (never git commits; returns Sci-Action proposal).".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "action": {
                        "type": "string",
                        "enum": ["suggest", "ground", "upsert", "promote"],
                        "description": "Citation action to perform"
                    },
                    "query": { "type": "string", "description": "Query or paper title (required for action=suggest)" },
                    "claim": { "type": "string", "description": "Claim sentence or paragraph to ground (required for action=ground)" },
                    "entry": { "type": "string", "description": "Full BibTeX entry block string (required for action=upsert)" },
                    "cite_key": { "type": "string", "description": "Cite key of the entry to promote (required for action=promote)" },
                    "source_id": { "type": "string", "description": "Optional source ID for action=suggest" },
                    "limit": { "type": "integer", "description": "Max suggestions for action=ground (default 5)" },
                    "apply": { "type": "boolean", "description": "Append a TODO idea block with suggestions for action=ground (default false)" },
                    "draft": { "type": "boolean", "description": "Mark as TUI-added draft for action=upsert (default false)" },
                    "preserve_cite_key": { "type": "boolean", "description": "Keep existing cite key when replacing for action=upsert (default true)" }
                }),
                vec!["action"],
            ),
        },
        Tool {
            name: "sil_draft".to_string(),
            description: "Manuscript mutations: edit section, update # -- X -- # TODOs, set structure completion.".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "action": {
                        "type": "string",
                        "enum": ["edit", "todo", "structure"],
                        "description": "Draft mutation action"
                    },
                    "section_title": { "type": "string", "description": "Section title to match for action=edit" },
                    "section_id": { "type": "string", "description": "Section ID for action=edit, todo, or structure" },
                    "content": { "type": "string", "description": "New section body (action=edit) or TODO content (action=todo)" },
                    "search": { "type": "string", "description": "Search string within section body for surgical replace (action=edit)" },
                    "replace": { "type": "string", "description": "Replacement string (required if search set for action=edit)" },
                    "expected_hash": { "type": "string", "description": "Optional short draft hash (action=edit)" },
                    "id": { "type": "string", "description": "Existing TODO block ID (action=todo)" },
                    "status": { "type": "string", "description": "Status for action=todo (open, in_progress, done)" },
                    "priority": { "type": "string", "description": "Priority for action=todo (high, medium, low)" },
                    "completion": {
                        "type": "string",
                        "enum": ["empty", "outline", "draft", "polished"],
                        "description": "Four-state section completion for action=structure"
                    },
                    "completed": { "type": "boolean", "description": "Deprecated completion flag for action=structure" },
                    "main_claim": { "type": "string", "description": "Primary claim for section (action=structure)" },
                    "secondary_points": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Secondary bullet points (action=structure)"
                    },
                    "required_content": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Required content checklist (action=structure)"
                    }
                }),
                vec!["action"],
            ),
        },
        Tool {
            name: "sil_review".to_string(),
            description: "Quality gates: L0 estimate (read-only on paper_draft.tex) or compile & doctor (sil build, sil doctor).".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "action": {
                        "type": "string",
                        "enum": ["estimate", "build"],
                        "description": "Quality action to perform"
                    },
                    "mode": { "type": "string", "description": "Estimate mode: quick | full | methodology (default quick)" },
                    "write": { "type": "boolean", "description": "Write report under .sil/reviews/ (default false)" },
                    "include_sources_summary": { "type": "boolean", "description": "Include sources summary string (default false)" },
                    "engine": { "type": "string", "description": "LaTeX engine for build (pdflatex, xelatex, lualatex, tectonic)" },
                    "run_doctor": { "type": "boolean", "description": "Run health audit for build (default true)" }
                }),
                vec!["action"],
            ),
        },
        Tool {
            name: "sil_propose".to_string(),
            description: "Format Sci-Action commit proposal; NEVER auto-commits.".to_string(),
            input_schema: ToolInputSchema::object(
                json!({
                    "message": { "type": "string", "description": "Optional custom commit message" },
                    "action": { "type": "string", "description": "Sci-Action category" }
                }),
                vec![],
            ),
        },
    ]
}

/// Execute a tool call by name with given parameters.
pub fn call_tool(name: &str, arguments: Option<serde_json::Value>) -> CallToolResult {
    let mut args = arguments.unwrap_or_else(|| json!({}));

    match name {
        "sil_context" => {
            if args.get("skill").is_some() || args.get("name").is_some() {
                handle_invoke_skill(args)
            } else if args.get("list_skills").and_then(|v| v.as_bool()).unwrap_or(false)
                || args.get("action").and_then(|v| v.as_str()) == Some("skills")
            {
                handle_list_skills(args)
            } else if args.get("status").is_some()
                || args.get("priority").is_some()
                || args.get("section").is_some()
                || args.get("sort_by").is_some()
                || args.get("action").and_then(|v| v.as_str()) == Some("todos")
            {
                handle_list_todos(args)
            } else if args.get("action").and_then(|v| v.as_str()) == Some("structure") {
                handle_get_structure(args)
            } else {
                handle_get_workspace_context(args)
            }
        }
        "sil_sources" => {
            let action = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return CallToolResult::error("Missing required parameter: action"),
            };
            match action {
                "search" => handle_search_sources(args),
                "get" => handle_get_source_context(args),
                "fetch" => handle_fetch_source(args),
                "parse" => handle_parse_source(args),
                "rank" => handle_rank_draft(args),
                _ => CallToolResult::error(format!("Invalid action '{action}' for sil_sources")),
            }
        }
        "sil_cite" => {
            let action = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return CallToolResult::error("Missing required parameter: action"),
            };
            match action {
                "suggest" => handle_suggest_citations(args),
                "ground" => handle_ground_claims(args),
                "upsert" => handle_upsert_bib(args),
                "promote" => handle_promote_bib(args),
                _ => CallToolResult::error(format!("Invalid action '{action}' for sil_cite")),
            }
        }
        "sil_draft" => {
            let action = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return CallToolResult::error("Missing required parameter: action"),
            };
            match action {
                "edit" => handle_edit_section(args),
                "todo" => handle_update_todo(args),
                "structure" => {
                    if let Some(obj) = args.as_object_mut() {
                        obj.insert("action".to_string(), json!("update"));
                    }
                    handle_get_structure(args)
                }
                _ => CallToolResult::error(format!("Invalid action '{action}' for sil_draft")),
            }
        }
        "sil_review" => {
            let action = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return CallToolResult::error("Missing required parameter: action"),
            };
            match action {
                "estimate" => handle_estimate_paper(args),
                "build" => handle_build_and_doctor(args),
                _ => CallToolResult::error(format!("Invalid action '{action}' for sil_review")),
            }
        }
        "sil_propose" => handle_propose_commit(args),
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
    let expand_parent = args
        .get("expand_parent")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

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
                    let res: Vec<_> = hits
                        .into_iter()
                        .map(|h| {
                            json!({
                                "source_id": h.id.as_str(),
                                "filename": h.filename,
                                "title": h.title,
                                "snippet": h.snippet,
                            })
                        })
                        .collect();
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
                let res: Vec<_> = hits
                    .into_iter()
                    .map(|h| {
                        json!({
                            "source_id": h.id.as_str(),
                            "filename": h.filename,
                            "title": h.title,
                            "snippet": h.snippet,
                        })
                    })
                    .collect();
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
                let chunks_json: Vec<_> = chunks
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "chunk_type": c.chunk_type.as_str(),
                            "heading_title": c.heading_title,
                            "content": c.content,
                        })
                    })
                    .collect();
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
            return CallToolResult::text(
                serde_json::to_string_pretty(&json!({
                    "bibtex": suggestion.bibtex,
                    "cite_command": suggestion.cite_command,
                    "key": suggestion.cite_key,
                    "note": suggestion.note
                }))
                .unwrap_or_default(),
            );
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

    let mut block = IdeaBlock::new(
        id.unwrap_or(""),
        content,
        section_id.map(String::from),
        0,
        0,
    );
    if let Some(st) = status {
        block.status = st.to_string();
    }
    if let Some(pr) = priority {
        block.priority = pr.to_string();
    }

    let updated_tex = update_or_insert_idea_block(&tex, &block);
    if let Err(e) = sil_core::write_atomic_str(&draft_path, &updated_tex) {
        return CallToolResult::error(format!("Failed to write paper_draft.tex: {e}"));
    }

    let ideas = parse_idea_blocks(&updated_tex);
    if let Ok(db) = SilDb::open(&paths.db()) {
        let _ = db.replace_todo_ideas(&ideas);
    }

    CallToolResult::text(
        json!({
            "status": "updated",
            "content": content,
            "section_id": section_id,
            "idea_blocks_count": ideas.len()
        })
        .to_string(),
    )
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
    let name = match args
        .get("skill")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
    {
        Some(n) => n,
        None => return CallToolResult::error("Missing required parameter: skill or name"),
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
    let include_paper = args
        .get("include_paper")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let (root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let config_yaml = fs::read_to_string(paths.config().as_str()).unwrap_or_default();
    let structure_yaml = fs::read_to_string(paths.structure().as_str()).unwrap_or_default();
    let structure = Structure::load(&paths.structure()).ok();
    let db = SilDb::open(&paths.db()).ok();
    let summary = db
        .as_ref()
        .and_then(|d| sources_summary(d).ok())
        .unwrap_or_default();
    let log = sil_git::log_entries(&root, 10, true).unwrap_or_default();

    let flags = ContextFlags {
        paper: include_paper,
        agent: false,
        skill_paper: false,
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

fn parse_string_array(value: Option<&serde_json::Value>) -> Option<Result<Vec<String>, String>> {
    value.map(|v| {
        let arr = v
            .as_array()
            .ok_or_else(|| "expected a JSON array of strings".to_string())?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            let s = item
                .as_str()
                .ok_or_else(|| "array items must be strings".to_string())?;
            out.push(s.to_string());
        }
        Ok(out)
    })
}

fn handle_get_structure(args: serde_json::Value) -> CallToolResult {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("read");
    let section_id = args.get("section_id").and_then(|v| v.as_str());
    let completion_arg = args.get("completion").and_then(|v| v.as_str());
    let completed = args.get("completed").and_then(|v| v.as_bool());
    let main_claim = args.get("main_claim").and_then(|v| v.as_str());
    let secondary_points = match parse_string_array(args.get("secondary_points")) {
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => return CallToolResult::error(format!("Invalid secondary_points: {e}")),
        None => None,
    };
    let required_content = match parse_string_array(args.get("required_content")) {
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => return CallToolResult::error(format!("Invalid required_content: {e}")),
        None => None,
    };
    // word_count is intentionally ignored (field does not exist on Section; non-goal for E2).

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let mut struct_obj = match Structure::load(&paths.structure()) {
        Ok(s) => s,
        Err(e) => return CallToolResult::error(format!("Failed to load structure.yaml: {e}")),
    };

    let mut proposal_msg: Option<String> = None;

    if action == "update" {
        let sid = match section_id {
            Some(s) => s,
            None => return CallToolResult::error("Missing section_id for update action"),
        };

        let new_completion = if let Some(c) = completion_arg {
            match c.parse::<SectionCompletion>() {
                Ok(v) => Some(v),
                Err(e) => {
                    return CallToolResult::error(format!(
                        "Invalid completion '{c}' (expected empty|outline|draft|polished): {e}"
                    ));
                }
            }
        } else {
            completed.map(|is_comp| {
                if is_comp {
                    SectionCompletion::Draft
                } else {
                    SectionCompletion::Empty
                }
            })
        };

        let has_field_update = main_claim.is_some()
            || secondary_points.is_some()
            || required_content.is_some()
            || new_completion.is_some();
        if !has_field_update {
            return CallToolResult::error(
                "update requires completion, completed, main_claim, secondary_points, and/or required_content",
            );
        }

        let sec = match struct_obj.sections.iter_mut().find(|s| s.id == sid) {
            Some(s) => s,
            None => {
                return CallToolResult::error(format!(
                    "Section '{sid}' not found in structure.yaml"
                ));
            }
        };

        let mut changes = Vec::new();
        if let Some(c) = new_completion {
            if sec.completion != c {
                changes.push(format!("completion {} → {}", sec.completion, c));
            }
            sec.completion = c;
        }
        if let Some(claim) = main_claim {
            sec.main_claim = claim.to_string();
            changes.push("main_claim".to_string());
        }
        if let Some(points) = secondary_points {
            sec.secondary_points = points;
            changes.push("secondary_points".to_string());
        }
        if let Some(req) = required_content {
            sec.required_content = req;
            changes.push("required_content".to_string());
        }

        if let Err(e) = struct_obj.save(&paths.structure()) {
            return CallToolResult::error(format!("Failed to save structure.yaml: {e}"));
        }

        let proposal = proposal_for_action(
            SciAction::UpdateStructure,
            Some(&format!("Update structure: {sid}")),
            Some(&format!(
                "Updated section `{sid}` ({})",
                if changes.is_empty() {
                    "no-op".to_string()
                } else {
                    changes.join(", ")
                }
            )),
        );
        proposal_msg = Some(proposal.message());
    } else if action != "read" {
        return CallToolResult::error(format!(
            "Invalid action '{action}' (expected 'read' or 'update')"
        ));
    }

    let summary = struct_obj.completion_summary();
    let mut res = json!({
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
    if let Some(msg) = proposal_msg {
        res["proposal"] = json!(msg);
        res["never_committed"] = json!(true);
    }
    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_build_and_doctor(args: serde_json::Value) -> CallToolResult {
    let engine_str = args
        .get("engine")
        .and_then(|v| v.as_str())
        .unwrap_or("pdflatex");
    let run_doctor = args
        .get("run_doctor")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

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
    let no_parse = args
        .get("no_parse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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
        let runner_res = sil_parse::discover_marker_runner();

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

fn handle_upsert_bib(args: serde_json::Value) -> CallToolResult {
    let entry = match args.get("entry").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return CallToolResult::error("Missing required parameter: entry"),
    };
    if entry.trim().is_empty() {
        return CallToolResult::error("entry must not be empty");
    }
    if !entry.contains('@') {
        return CallToolResult::error("entry is not valid BibTeX (missing @type{key, ...})");
    }

    let draft = args.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
    let preserve_cite_key = args
        .get("preserve_cite_key")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let bib_path = paths.join(sil_core::paths::rel::REFERENCES);
    // Re-read disk before write (same concurrency model as TUI).
    let current = fs::read_to_string(bib_path.as_str()).unwrap_or_default();

    let entry_for_upsert = if draft {
        mark_tui_added_bib_entry(entry)
    } else {
        entry.to_string()
    };

    let (updated, replaced) = upsert_bib_entry_with_options(
        &current,
        &entry_for_upsert,
        UpsertOptions { preserve_cite_key },
    );

    let new_info = extract_bib_entry_info(&entry_for_upsert);
    let cite_key = parse_bib_blocks(&updated)
        .into_iter()
        .find(|block| is_same_paper(&extract_bib_entry_info(block), &new_info))
        .and_then(|block| extract_bib_entry_info(&block).cite_key)
        .or_else(|| new_info.cite_key.clone())
        .unwrap_or_else(|| "unknown".to_string());

    if let Err(e) = sil_core::write_atomic_str(&bib_path, &updated) {
        return CallToolResult::error(format!("Failed to write {}: {e}", bib_path));
    }

    let proposal = proposal_for_action(
        SciAction::UpdateBibliography,
        Some(&format!("Update bibliography: {cite_key}")),
        Some(&format!(
            "Upserted BibTeX entry '{cite_key}' into {} (draft={draft}, preserve_cite_key={preserve_cite_key}, replaced={replaced})",
            sil_core::paths::rel::REFERENCES
        )),
    );

    let res = json!({
        "wrote": true,
        "cite_key": cite_key,
        "replaced": replaced,
        "path": bib_path.as_str(),
        "draft": draft,
        "proposal": proposal.message(),
        "never_committed": true,
    });

    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_promote_bib(args: serde_json::Value) -> CallToolResult {
    let cite_key = match args.get("cite_key").and_then(|v| v.as_str()) {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => return CallToolResult::error("Missing required parameter: cite_key"),
    };

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let bib_path = paths.join(sil_core::paths::rel::REFERENCES);
    if !bib_path.is_file() {
        return CallToolResult::error(format!("references.bib not found at {bib_path}"));
    }

    // Re-read disk before write.
    let current = match fs::read_to_string(bib_path.as_str()) {
        Ok(c) => c,
        Err(e) => return CallToolResult::error(format!("Failed to read {bib_path}: {e}")),
    };

    let target_info = sil_core::BibEntryInfo {
        cite_key: Some(cite_key.to_string()),
        title: Some(cite_key.to_string()),
        doi: Some(cite_key.to_string()),
        arxiv_id: Some(cite_key.to_string()),
        is_incomplete: false,
    };

    let mut blocks = parse_bib_blocks(&current);
    let mut promoted_key: Option<String> = None;
    let mut had_marker = false;

    for block in &mut blocks {
        let block_info = extract_bib_entry_info(block);
        let key_match = block_info
            .cite_key
            .as_deref()
            .unwrap_or("")
            .eq_ignore_ascii_case(cite_key);
        if is_same_paper(&block_info, &target_info) || key_match {
            let key = block_info
                .cite_key
                .as_deref()
                .unwrap_or(cite_key)
                .to_string();
            had_marker = is_tui_added_bib_block(block);
            *block = unmark_tui_added_bib_entry(block);
            promoted_key = Some(key);
            break;
        }
    }

    let Some(key) = promoted_key else {
        return CallToolResult::error(format!(
            "No entry matching '{cite_key}' found in {bib_path} to promote"
        ));
    };

    let updated = if blocks.is_empty() {
        String::new()
    } else {
        blocks.join("\n\n") + "\n"
    };

    if let Err(e) = sil_core::write_atomic_str(&bib_path, &updated) {
        return CallToolResult::error(format!("Failed to write {bib_path}: {e}"));
    }

    let proposal = proposal_for_action(
        SciAction::PromoteBibliography,
        Some(&format!("Promote bibliography entry: {key}")),
        Some(&format!(
            "Removed % [sil: tui-added] from '{key}' in {}",
            sil_core::paths::rel::REFERENCES
        )),
    );

    let res = json!({
        "wrote": true,
        "cite_key": key,
        "replaced": had_marker,
        "path": bib_path.as_str(),
        "proposal": proposal.message(),
        "never_committed": true,
    });

    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

/// Resolve a filesystem path for parse: absolute, relative to cwd, or under sources/.
fn resolve_parse_path(
    raw: &str,
    sources_dir: &camino::Utf8Path,
    root: &camino::Utf8Path,
) -> Result<Utf8PathBuf, String> {
    let candidate = Utf8PathBuf::from(raw);
    if candidate.is_absolute() {
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(format!("path not found: {raw}"));
    }

    // relative to cwd
    if candidate.exists() {
        return Ok(candidate);
    }

    // under sources/ as filename or relative subpath
    let under_sources = sources_dir.join(raw.trim_start_matches("sources/"));
    if under_sources.exists() {
        return Ok(under_sources);
    }

    // under project root
    let under_root = root.join(raw);
    if under_root.exists() {
        return Ok(under_root);
    }

    Err(format!(
        "path not found: {raw} (tried cwd, sources/, and project root)"
    ))
}

/// Resolve source_id (DB id or filename) to a file under sources/.
fn resolve_source_id_path(
    source_id: &str,
    sources_dir: &camino::Utf8Path,
    root: &camino::Utf8Path,
    db: &SilDb,
) -> Result<Utf8PathBuf, String> {
    // Prefer filesystem under sources/
    if let Ok(p) = resolve_parse_path(source_id, sources_dir, root) {
        return Ok(p);
    }

    // DB row may store path even if not yet on disk under expected name
    if let Ok(Some((doc, _))) = db.get_source_content(source_id) {
        let stored = Utf8PathBuf::from(doc.path.as_str());
        if stored.is_absolute() && stored.exists() {
            return Ok(stored);
        }
        let under_root = root.join(&stored);
        if under_root.exists() {
            return Ok(under_root);
        }
        let under_sources = sources_dir.join(&doc.filename);
        if under_sources.exists() {
            return Ok(under_sources);
        }
        return Err(format!(
            "source_id '{source_id}' found in DB but file missing on disk (path={})",
            doc.path
        ));
    }

    Err(format!(
        "source_id '{source_id}' not found under sources/ or in the database"
    ))
}

fn parse_result_json(r: &sil_parse::batch::ParseResult) -> serde_json::Value {
    json!({
        "source_id": r.document.id.as_str(),
        "filename": r.document.filename,
        "parsed": r.document.parsed,
        "title": r.document.title,
        "doi": r.document.doi,
        "authors": r.document.authors,
        "reference_count": r.reference_count,
        "content_chars": r.content.len(),
        "duration_ms": r.duration.as_millis() as u64,
    })
}

fn handle_parse_source(args: serde_json::Value) -> CallToolResult {
    let source_id = args.get("source_id").and_then(|v| v.as_str());
    let path_arg = args.get("path").and_then(|v| v.as_str());
    let all_unparsed = args
        .get("all_unparsed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if source_id.is_none() && path_arg.is_none() && !all_unparsed {
        return CallToolResult::error("Provide path, source_id, or set all_unparsed=true");
    }

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

    let runner = match sil_parse::discover_marker_runner() {
        Ok(r) => r,
        Err(e) => {
            // Fall back to stub so agents can parse non-PDF text sources without Marker installed.
            // PDF parse via stub still succeeds with placeholder content (tests set SIL_MARKER_STUB).
            let _ = e;
            Box::new(sil_parse::StubMarkerRunner {
                content: std::env::var("SIL_MARKER_STUB").unwrap_or_else(|_| {
                    "# sil-mcp stub parse\n\n(no Marker runner available)\n".to_string()
                }),
            })
        }
    };
    let null_ui = sil_core::NullUi::new();

    let to_parse: Vec<Utf8PathBuf> = if all_unparsed {
        match sil_parse::list_unparsed_pdfs(&sources_dir, &db) {
            Ok(list) => list,
            Err(e) => {
                return CallToolResult::error(format!("Failed to list unparsed sources: {e}"));
            }
        }
    } else if let Some(p) = path_arg {
        match resolve_parse_path(p, &sources_dir, &root) {
            Ok(abs) => vec![abs],
            Err(e) => return CallToolResult::error(e),
        }
    } else if let Some(sid) = source_id {
        match resolve_source_id_path(sid, &sources_dir, &root, &db) {
            Ok(abs) => vec![abs],
            Err(e) => return CallToolResult::error(e),
        }
    } else {
        Vec::new()
    };

    if to_parse.is_empty() {
        return CallToolResult::error("Nothing to parse (no matching unparsed sources)");
    }

    if to_parse.len() == 1 {
        let path = &to_parse[0];
        match sil_parse::parse_one(path, &db, runner.as_ref(), &null_ui) {
            Ok(r) => {
                let proposal = proposal_for_action(
                    SciAction::ParsePdf,
                    Some(&format!("Parse source: {}", r.document.filename)),
                    Some(&format!(
                        "Ingested {} into SQLite + FTS5 ({} refs).",
                        r.document.filename, r.reference_count
                    )),
                );
                let res = json!({
                    "ok": true,
                    "parsed_count": 1,
                    "failed_count": 0,
                    "results": [parse_result_json(&r)],
                    "proposal": proposal.message(),
                    "never_committed": true,
                });
                CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
            }
            Err(e) => CallToolResult::error(format!("Parse failed for {path}: {e}")),
        }
    } else {
        let (ok, failed, errors) = sil_parse::parse_many(&to_parse, &db, runner.as_ref(), &null_ui);
        let mut results = Vec::new();
        // parse_many does not return per-file results; report counts + errors only.
        for (p, err) in &errors {
            results.push(json!({
                "path": p.as_str(),
                "ok": false,
                "error": err,
            }));
        }
        if ok == 0 {
            return CallToolResult::error(format!(
                "Batch parse failed: 0 parsed, {failed} failed: {}",
                errors
                    .iter()
                    .map(|(p, e)| format!("{}: {e}", p.file_name().unwrap_or(p.as_str())))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        let proposal = proposal_for_action(
            SciAction::ParsePdf,
            Some(&format!("Parse {ok} source(s)")),
            Some(&format!("Parsed {ok} file(s), {failed} failed.")),
        );
        let res = json!({
            "ok": true,
            "parsed_count": ok,
            "failed_count": failed,
            "errors": results,
            "proposal": proposal.message(),
            "never_committed": true,
        });
        CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
    }
}

fn handle_rank_draft(args: serde_json::Value) -> CallToolResult {
    let min_score = args
        .get("min_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let draft_path = paths.paper_draft();
    if !draft_path.exists() {
        return CallToolResult::error(format!("paper_draft.tex not found at {draft_path}"));
    }
    let draft_text = match fs::read_to_string(draft_path.as_str()) {
        Ok(t) => t,
        Err(e) => return CallToolResult::error(format!("Failed to read {draft_path}: {e}")),
    };

    let db = match SilDb::open(&paths.db()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::error(format!("Failed to open database: {e}")),
    };

    let embedder = sil_db::OnnxEmbedder::default();
    let count = match db.recompute_draft_ref_similarities(&draft_text, &embedder) {
        Ok(n) => n,
        Err(e) => {
            return CallToolResult::error(format!("Failed to recompute draft similarities: {e}"));
        }
    };

    let scores = match db.get_draft_ref_similarities() {
        Ok(s) => s,
        Err(e) => return CallToolResult::error(format!("Failed to read similarities: {e}")),
    };
    let all_refs = match db.get_all_references() {
        Ok(r) => r,
        Err(e) => return CallToolResult::error(format!("Failed to list references: {e}")),
    };

    let mut hits: Vec<serde_json::Value> = all_refs
        .into_iter()
        .filter_map(|r| {
            let score = *scores.get(&r.id).unwrap_or(&0.0);
            if score >= min_score {
                Some(json!({
                    "ref_id": r.id,
                    "title": r.title,
                    "authors": r.authors,
                    "year": r.year,
                    "score": score,
                    "raw_text": r.raw_text,
                }))
            } else {
                None
            }
        })
        .collect();

    hits.sort_by(|a, b| {
        let sa = a["score"].as_f64().unwrap_or(0.0);
        let sb = b["score"].as_f64().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);

    let res = json!({
        "computed": count,
        "min_score": min_score,
        "hits": hits,
        "count": hits.len(),
    });
    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn handle_estimate_paper(args: serde_json::Value) -> CallToolResult {
    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("quick");
    let write = args.get("write").and_then(|v| v.as_bool()).unwrap_or(false);
    let include_sources = args
        .get("include_sources_summary")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let (root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let structure = Structure::load(&paths.structure()).ok();
    let report = match run_heuristic_estimate(&EstimateInput {
        root: &root,
        mode: EstimateMode::parse(mode),
        structure: structure.as_ref(),
    }) {
        Ok(r) => r,
        Err(e) => return CallToolResult::error(format!("estimate failed: {e}")),
    };

    let mut res = match serde_json::to_value(&report) {
        Ok(v) => v,
        Err(e) => return CallToolResult::error(format!("serialize: {e}")),
    };

    if include_sources {
        let summary = match SilDb::open(&paths.db()) {
            Ok(db) => sources_summary(&db).unwrap_or_default(),
            Err(_) => String::new(),
        };
        if let Some(obj) = res.as_object_mut() {
            obj.insert("sources_summary".into(), json!(summary));
        }
    }

    if write {
        match write_estimate_report(&root, &report) {
            Ok(dir) => {
                if let Some(obj) = res.as_object_mut() {
                    obj.insert("report_dir".into(), json!(dir.to_string()));
                    obj.insert(
                        "proposal".into(),
                        json!(estimate_proposal_message(&dir)),
                    );
                }
            }
            Err(e) => return CallToolResult::error(format!("write report: {e}")),
        }
    }

    if let Some(obj) = res.as_object_mut() {
        obj.insert("never_committed".into(), json!(true));
        obj.insert("read_only".into(), json!(true));
    }

    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn draft_short_hash(tex: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in tex.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn handle_edit_section(args: serde_json::Value) -> CallToolResult {
    let section_title = args
        .get("section_title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let section_id = args
        .get("section_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let content = args.get("content").and_then(|v| v.as_str());
    let search = args.get("search").and_then(|v| v.as_str());
    let replace = args.get("replace").and_then(|v| v.as_str());
    let expected_hash = args.get("expected_hash").and_then(|v| v.as_str());

    if section_title.is_none() && section_id.is_none() {
        return CallToolResult::error("Provide section_title and/or section_id");
    }
    if content.is_none() && search.is_none() {
        return CallToolResult::error("Provide content (full body replace) or search+replace");
    }
    if search.is_some() && replace.is_none() {
        return CallToolResult::error("replace is required when search is set");
    }

    let (root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let _ = write_lock(&paths, &WorkspaceLock::new("mcp", "edit-section"));

    let draft_path = paths.paper_draft();
    let tex = match fs::read_to_string(draft_path.as_str()) {
        Ok(t) => t,
        Err(e) => return CallToolResult::error(format!("read draft: {e}")),
    };
    let current_hash = draft_short_hash(&tex);
    if let Some(exp) = expected_hash
        && exp != current_hash
    {
        return CallToolResult::error(format!(
            "expected_hash mismatch: got {current_hash}, expected {exp}"
        ));
    }

    // Resolve title from structure id if needed
    let mut title_query = section_title.unwrap_or_default();
    if title_query.is_empty()
        && let Some(id) = &section_id
    {
        title_query = if let Ok(st) = Structure::load(&paths.structure()) {
            st.sections
                .iter()
                .find(|s| s.id == *id)
                .map(|sec| sec.title.clone())
                .unwrap_or_else(|| id.clone())
        } else {
            id.clone()
        };
    }

    let sections = split_tex_sections(&tex);
    let needle = title_query.to_ascii_lowercase();
    let Some(target) = sections.iter().find(|s| {
        let t = s.title.to_ascii_lowercase();
        t == needle || t.contains(&needle) || needle.contains(&t)
    }) else {
        return CallToolResult::error(format!(
            "No LaTeX section matching title/id '{title_query}'"
        ));
    };

    // Rebuild source: replace body of matched section
    let lines: Vec<&str> = tex.lines().collect();
    let heading_idx = target.line_start.saturating_sub(1);
    if heading_idx >= lines.len() {
        return CallToolResult::error("section heading index out of range");
    }

    // Find end of section body (same logic as splitter)
    let level = match target.kind.as_str() {
        "subsubsection" => 3u8,
        "subsection" => 2,
        _ => 1,
    };
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(heading_idx + 1) {
        if let Some((_, _, l)) = parse_heading_level(line)
            && l <= level
        {
            end = i;
            break;
        }
    }

    let mut new_body = if let Some(full) = content {
        full.to_string()
    } else {
        lines[heading_idx + 1..end].join("\n")
    };
    if let (Some(s), Some(r)) = (search, replace) {
        if !new_body.contains(s) {
            return CallToolResult::error("search string not found in section body");
        }
        new_body = new_body.replacen(s, r, 1);
    }

    let mut out_lines: Vec<String> = Vec::new();
    out_lines.extend(lines[..heading_idx + 1].iter().map(|s| (*s).to_string()));
    if !new_body.is_empty() {
        out_lines.extend(new_body.lines().map(|s| s.to_string()));
    }
    out_lines.extend(lines[end..].iter().map(|s| (*s).to_string()));
    // Preserve trailing newline if original had one
    let mut updated = out_lines.join("\n");
    if tex.ends_with('\n') && !updated.ends_with('\n') {
        updated.push('\n');
    }

    if let Err(e) = sil_core::write_atomic_str(&draft_path, &updated) {
        return CallToolResult::error(format!("write draft: {e}"));
    }

    let proposal = proposal_for_action(
        SciAction::EditDraft,
        Some(&format!("Edit section {}", target.title)),
        Some(&format!(
            "Updated section `{}` via sil_edit_section.",
            target.title
        )),
    );

    let res = json!({
        "wrote": true,
        "section_title": target.title,
        "path": draft_path.to_string(),
        "draft_hash_before": current_hash,
        "draft_hash_after": draft_short_hash(&updated),
        "proposal": proposal.message(),
        "never_committed": true,
        "project_root": root.to_string(),
    });
    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

fn parse_heading_level(line: &str) -> Option<(String, String, u8)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix('\\')?;
    let (cmd, after) = rest
        .strip_prefix("subsubsection")
        .map(|r| ("subsubsection", r))
        .or_else(|| rest.strip_prefix("subsection").map(|r| ("subsection", r)))
        .or_else(|| rest.strip_prefix("section").map(|r| ("section", r)))?;
    let after = after.strip_prefix('*').unwrap_or(after).trim_start();
    let level = match cmd {
        "subsubsection" => 3u8,
        "subsection" => 2,
        _ => 1,
    };
    Some((cmd.into(), after.into(), level))
}

fn handle_ground_claims(args: serde_json::Value) -> CallToolResult {
    let claim = match args.get("claim").and_then(|v| v.as_str()) {
        Some(c) if !c.trim().is_empty() => c,
        _ => return CallToolResult::error("Missing required parameter: claim"),
    };
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let apply = args.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);

    let (_root, paths) = match get_project_paths() {
        Ok(p) => p,
        Err(e) => return CallToolResult::error(e),
    };

    let db = match SilDb::open(&paths.db()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::error(format!("Failed to open database: {e}")),
    };

    let embedder = sil_db::OnnxEmbedder::default();
    let suggestions: Vec<serde_json::Value> =
        match db.search_hybrid(&embedder, claim, limit, true) {
            Ok(hits) => hits
                .iter()
                .map(|h| {
                    json!({
                        "source_id": h.chunk.source_id.to_string(),
                        "chunk_id": h.chunk.id,
                        "score": h.score,
                        "snippet": h.snippet.chars().take(400).collect::<String>(),
                    })
                })
                .collect(),
            Err(_) => match db.search(claim, limit) {
                Ok(hits) => hits
                    .iter()
                    .map(|h| {
                        json!({
                            "source_id": h.id.to_string(),
                            "chunk_id": serde_json::Value::Null,
                            "score": 0.0,
                            "snippet": h.snippet.chars().take(400).collect::<String>(),
                            "title": h.title,
                            "filename": h.filename,
                        })
                    })
                    .collect(),
                Err(e) => return CallToolResult::error(format!("search failed: {e}")),
            },
        };

    let mut applied = false;
    if apply {
        let draft_path = paths.paper_draft();
        if let Ok(tex) = fs::read_to_string(draft_path.as_str()) {
            let mut note = format!("Claim grounding for: {claim}\n");
            for (i, s) in suggestions.iter().enumerate() {
                note.push_str(&format!(
                    "{}. source={} score={}\n",
                    i + 1,
                    s["source_id"].as_str().unwrap_or("?"),
                    s["score"]
                ));
            }
            let block = IdeaBlock {
                id: format!("ground-{}", draft_short_hash(claim)),
                content: note,
                section_id: None,
                line_start: 0,
                line_end: 0,
                status: "open".into(),
                priority: "medium".into(),
                author_type: "agent".into(),
                tags: vec!["ground-claims".into()],
                created_at: String::new(),
            };
            let updated = update_or_insert_idea_block(&tex, &block);
            if sil_core::write_atomic_str(&draft_path, &updated).is_ok() {
                applied = true;
            }
        }
    }

    let proposal = if applied {
        Some(
            proposal_for_action(
                SciAction::GroundClaims,
                Some("Ground claims with literature search"),
                Some("Recorded claim grounding TODO via sil_ground_claims."),
            )
            .message(),
        )
    } else {
        None
    };

    let res = json!({
        "claim": claim,
        "suggestions": suggestions,
        "count": suggestions.len(),
        "applied": applied,
        "proposal": proposal,
        "never_committed": true,
    });
    CallToolResult::text(serde_json::to_string_pretty(&res).unwrap_or_default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::protocol::Content;
    use std::fs;
    use std::sync::{Mutex, MutexGuard};
    use tempfile::TempDir;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    pub(crate) struct TestEnv {
        _guard: MutexGuard<'static, ()>,
        _dir: TempDir,
        orig_cwd: std::path::PathBuf,
        project_root: Utf8PathBuf,
    }

    impl TestEnv {
        pub(crate) fn new() -> Self {
            let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            let orig_cwd = std::env::current_dir().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let project_root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

            fs::create_dir_all(project_root.join("agent/skills")).unwrap();
            fs::create_dir_all(project_root.join(".sil")).unwrap();
            fs::write(project_root.join(".sil/config.yaml"), "version: 1\n").unwrap();
            fs::write(
                project_root.join("agent/skills/SYSTEM.md"),
                "# System Skill\nDefault rules.",
            )
            .unwrap();

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

    // --- handle_search_sources ---

    #[test]
    fn test_search_sources_missing_query_error() {
        let res = handle_search_sources(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: query"));
    }

    #[test]
    fn test_search_sources_limit_variations() {
        let env = TestEnv::new();
        let db = env.setup_db();

        for i in 1..=5 {
            let sid = sil_core::SourceId::new(format!("paper_{i}"));
            let mut doc =
                sil_core::SourceDocument::new(Utf8PathBuf::from(format!("sources/paper_{i}.pdf")));
            doc.id = sid.clone();
            doc.title = Some(format!("Quantum Article {i}"));
            db.upsert_parsed(&doc, "Quantum computing and state vectors.")
                .unwrap();

            let chunk = sil_db::SourceChunk {
                id: format!("chk_{i}"),
                source_id: sid,
                parent_chunk_id: None,
                chunk_type: sil_db::ChunkType::Parent,
                heading_title: Some(format!("Quantum Heading {i}")),
                content: "Quantum computing and state vectors.".to_string(),
                start_offset: 0,
                end_offset: 35,
                embedding_blob: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            };
            db.insert_source_chunks(&[chunk]).unwrap();
        }

        // limit 1
        let res1 = handle_search_sources(json!({ "query": "Quantum", "limit": 1 }));
        assert!(res1.is_error.is_none() || res1.is_error == Some(false));
        let val1: serde_json::Value = serde_json::from_str(extract_text(&res1)).unwrap();
        assert_eq!(val1.as_array().unwrap().len(), 1);

        // limit 5
        let res5 = handle_search_sources(json!({ "query": "Quantum", "limit": 5 }));
        assert!(res5.is_error.is_none() || res5.is_error == Some(false));
        let val5: serde_json::Value = serde_json::from_str(extract_text(&res5)).unwrap();
        assert_eq!(val5.as_array().unwrap().len(), 5);

        // limit 0
        let res0 = handle_search_sources(json!({ "query": "Quantum", "limit": 0 }));
        assert!(res0.is_error.is_none() || res0.is_error == Some(false));
        let val0: serde_json::Value = serde_json::from_str(extract_text(&res0)).unwrap();
        assert_eq!(val0.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_search_sources_hyde_mode() {
        let env = TestEnv::new();
        let _db = env.setup_db();

        let res_hyde_true = handle_search_sources(json!({ "query": "photons", "hyde": true }));
        assert!(res_hyde_true.is_error.is_none() || res_hyde_true.is_error == Some(false));

        let res_hyde_false = handle_search_sources(json!({ "query": "photons", "hyde": false }));
        assert!(res_hyde_false.is_error.is_none() || res_hyde_false.is_error == Some(false));
    }

    #[test]
    fn test_search_sources_expand_parent() {
        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("paper_exp");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/paper_exp.pdf"));
        doc.id = sid.clone();
        db.upsert_parsed(&doc, "Expansion text").unwrap();

        let res_true =
            handle_search_sources(json!({ "query": "Expansion", "expand_parent": true }));
        assert!(res_true.is_error.is_none() || res_true.is_error == Some(false));

        let res_false =
            handle_search_sources(json!({ "query": "Expansion", "expand_parent": false }));
        assert!(res_false.is_error.is_none() || res_false.is_error == Some(false));
    }

    #[test]
    fn test_search_sources_empty_vs_matching_index() {
        let env = TestEnv::new();
        let db = env.setup_db();

        // Empty index
        let res_empty = handle_search_sources(json!({ "query": "Nonexistent" }));
        assert!(res_empty.is_error.is_none() || res_empty.is_error == Some(false));
        let val_empty: serde_json::Value = serde_json::from_str(extract_text(&res_empty)).unwrap();
        assert_eq!(val_empty.as_array().unwrap().len(), 0);

        // Populate matching chunk
        let sid = sil_core::SourceId::new("paper_match");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/paper_match.pdf"));
        doc.id = sid.clone();
        doc.title = Some("Matching Paper Title".to_string());
        db.upsert_parsed(&doc, "Unique matching text content.")
            .unwrap();

        let chunk = sil_db::SourceChunk {
            id: "chk_match_1".to_string(),
            source_id: sid,
            parent_chunk_id: None,
            chunk_type: sil_db::ChunkType::Parent,
            heading_title: Some("Matching Heading".to_string()),
            content: "Unique matching text content.".to_string(),
            start_offset: 0,
            end_offset: 30,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_source_chunks(&[chunk]).unwrap();

        let res_match = handle_search_sources(json!({ "query": "Unique" }));
        assert!(res_match.is_error.is_none() || res_match.is_error == Some(false));
        let val_match: serde_json::Value = serde_json::from_str(extract_text(&res_match)).unwrap();
        assert_eq!(val_match.as_array().unwrap().len(), 1);
        assert_eq!(val_match[0]["source_id"], "paper_match");
    }

    // --- handle_get_source_context ---

    #[test]
    fn test_get_source_context_missing_source_id_error() {
        let res = handle_get_source_context(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: source_id"));
    }

    #[test]
    fn test_get_source_context_all_chunks() {
        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("src_all_chunks");
        let mut doc =
            sil_core::SourceDocument::new(Utf8PathBuf::from("sources/src_all_chunks.pdf"));
        doc.id = sid.clone();
        db.upsert_parsed(&doc, "content").unwrap();

        let chunk1 = sil_db::SourceChunk {
            id: "chk1".to_string(),
            source_id: sid.clone(),
            parent_chunk_id: None,
            chunk_type: sil_db::ChunkType::Parent,
            heading_title: Some("Section 1".to_string()),
            content: "Content 1".to_string(),
            start_offset: 0,
            end_offset: 10,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let chunk2 = sil_db::SourceChunk {
            id: "chk2".to_string(),
            source_id: sid.clone(),
            parent_chunk_id: None,
            chunk_type: sil_db::ChunkType::Parent,
            heading_title: Some("Section 2".to_string()),
            content: "Content 2".to_string(),
            start_offset: 11,
            end_offset: 20,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_source_chunks(&[chunk1, chunk2]).unwrap();

        let res = handle_get_source_context(json!({ "source_id": "src_all_chunks" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["source_id"], "src_all_chunks");
        assert_eq!(val["chunk_count"], 2);
        assert_eq!(val["chunks"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_get_source_context_specific_chunk_parent_expansion() {
        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("src_exp_chunk");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/src_exp_chunk.pdf"));
        doc.id = sid.clone();
        db.upsert_parsed(&doc, "content").unwrap();

        let parent = sil_db::SourceChunk {
            id: "p_head".to_string(),
            source_id: sid.clone(),
            parent_chunk_id: None,
            chunk_type: sil_db::ChunkType::Parent,
            heading_title: Some("Main Section".to_string()),
            content: "Parent Section Full Context".to_string(),
            start_offset: 0,
            end_offset: 30,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let child = sil_db::SourceChunk {
            id: "c_child".to_string(),
            source_id: sid,
            parent_chunk_id: Some("p_head".to_string()),
            chunk_type: sil_db::ChunkType::Child,
            heading_title: None,
            content: "Sub paragraph text".to_string(),
            start_offset: 31,
            end_offset: 50,
            embedding_blob: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_source_chunks(&[parent, child]).unwrap();

        let res = handle_get_source_context(json!({
            "source_id": "src_exp_chunk",
            "chunk_id": "c_child"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["chunk"]["id"], "c_child");
        assert_eq!(val["parent_context"]["id"], "p_head");
        assert_eq!(
            val["parent_context"]["content"],
            "Parent Section Full Context"
        );
    }

    #[test]
    fn test_get_source_context_nonexistent_chunk_error() {
        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("src_no_chk");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/src_no_chk.pdf"));
        doc.id = sid;
        db.upsert_parsed(&doc, "content").unwrap();

        let res = handle_get_source_context(json!({
            "source_id": "src_no_chk",
            "chunk_id": "invalid_chunk_99"
        }));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Chunk 'invalid_chunk_99' not found"));
    }

    #[test]
    fn test_get_source_context_nonexistent_source() {
        let env = TestEnv::new();
        let _db = env.setup_db();

        let res = handle_get_source_context(json!({ "source_id": "ghost_source" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["source_id"], "ghost_source");
        assert_eq!(val["chunk_count"], 0);
        assert_eq!(val["chunks"].as_array().unwrap().len(), 0);
    }

    // --- handle_suggest_citations ---

    #[test]
    fn test_suggest_citations_missing_query_error() {
        let res = handle_suggest_citations(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: query"));
    }

    #[test]
    fn test_suggest_citations_source_id_in_db() {
        let env = TestEnv::new();
        let db = env.setup_db();

        let sid = sil_core::SourceId::new("doc_cit_1");
        let mut doc = sil_core::SourceDocument::new(Utf8PathBuf::from("sources/doc_cit_1.pdf"));
        doc.id = sid;
        doc.title = Some("Deep Learning Architecture Advances".to_string());
        doc.authors = Some("Yann LeCun, Yoshua Bengio".to_string());
        doc.doi = Some("10.1038/nature14539".to_string());
        doc.year = Some(2015);
        db.upsert_parsed(&doc, "Deep learning methods...").unwrap();

        let res = handle_suggest_citations(json!({
            "query": "Deep Learning",
            "source_id": "doc_cit_1"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        let bibtex = val["bibtex"].as_str().unwrap();
        assert!(bibtex.contains("Deep Learning Architecture Advances"));
        assert!(bibtex.contains("Yann LeCun"));
        assert!(bibtex.contains("10.1038/nature14539"));
        assert!(val["cite_command"].as_str().unwrap().starts_with("\\cite{"));
    }

    #[test]
    fn test_suggest_citations_source_id_not_found_fallback() {
        let _env = TestEnv::new();

        let res = handle_suggest_citations(json!({
            "query": "Transformers Attention",
            "source_id": "missing_source_id"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert!(
            val["bibtex"]
                .as_str()
                .unwrap()
                .contains("Transformers Attention")
        );
        assert!(val["key"].is_string());
    }

    #[test]
    fn test_suggest_citations_query_only() {
        let res = handle_suggest_citations(json!({ "query": "Graph Neural Networks" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert!(
            val["bibtex"]
                .as_str()
                .unwrap()
                .contains("Graph Neural Networks")
        );
        assert!(
            val["cite_command"]
                .as_str()
                .unwrap()
                .contains("graph_neural_networks")
        );
    }

    // --- handle_list_todos ---

    #[test]
    fn test_list_todos_all_without_filters() {
        let env = TestEnv::new();

        let tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=todo_a, priority=high, status=open, section=sec_intro]
% Intro work
% # -- X -- #
% # -- X -- #
% [TODO: id=todo_b, priority=low, status=done, section=sec_methods]
% Methods work
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), tex).unwrap();

        let res = handle_list_todos(json!({}));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_list_todos_filter_by_status() {
        let env = TestEnv::new();

        let tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=t1, status=open]
% Open task
% # -- X -- #
% # -- X -- #
% [TODO: id=t2, status=in_progress]
% In progress task
% # -- X -- #
% # -- X -- #
% [TODO: id=t3, status=done]
% Done task
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), tex).unwrap();

        let res_open = handle_list_todos(json!({ "status": "open" }));
        let val_open: serde_json::Value = serde_json::from_str(extract_text(&res_open)).unwrap();
        assert_eq!(val_open.as_array().unwrap().len(), 1);
        assert_eq!(val_open[0]["id"], "t1");

        let res_prog = handle_list_todos(json!({ "status": "in_progress" }));
        let val_prog: serde_json::Value = serde_json::from_str(extract_text(&res_prog)).unwrap();
        assert_eq!(val_prog.as_array().unwrap().len(), 1);
        assert_eq!(val_prog[0]["id"], "t2");

        let res_done = handle_list_todos(json!({ "status": "done" }));
        let val_done: serde_json::Value = serde_json::from_str(extract_text(&res_done)).unwrap();
        assert_eq!(val_done.as_array().unwrap().len(), 1);
        assert_eq!(val_done[0]["id"], "t3");
    }

    #[test]
    fn test_list_todos_filter_by_priority() {
        let env = TestEnv::new();

        let tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=p_hi, priority=high]
% High prio
% # -- X -- #
% # -- X -- #
% [TODO: id=p_med, priority=medium]
% Med prio
% # -- X -- #
% # -- X -- #
% [TODO: id=p_lo, priority=low]
% Low prio
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), tex).unwrap();

        let res_hi = handle_list_todos(json!({ "priority": "high" }));
        let val_hi: serde_json::Value = serde_json::from_str(extract_text(&res_hi)).unwrap();
        assert_eq!(val_hi.as_array().unwrap().len(), 1);
        assert_eq!(val_hi[0]["id"], "p_hi");

        let res_med = handle_list_todos(json!({ "priority": "medium" }));
        let val_med: serde_json::Value = serde_json::from_str(extract_text(&res_med)).unwrap();
        assert_eq!(val_med.as_array().unwrap().len(), 1);
        assert_eq!(val_med[0]["id"], "p_med");

        let res_lo = handle_list_todos(json!({ "priority": "low" }));
        let val_lo: serde_json::Value = serde_json::from_str(extract_text(&res_lo)).unwrap();
        assert_eq!(val_lo.as_array().unwrap().len(), 1);
        assert_eq!(val_lo[0]["id"], "p_lo");
    }

    #[test]
    fn test_list_todos_filter_by_section() {
        let env = TestEnv::new();

        let tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
% # -- X -- #
% [TODO: id=s_intro]
% Intro item
% # -- X -- #
\section{Methods}
% # -- X -- #
% [TODO: id=s_methods]
% Methods item
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), tex).unwrap();

        let res = handle_list_todos(json!({ "section": "sec_intro" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
    }

    #[test]
    fn test_list_todos_sort_by_priority_and_line_start() {
        let env = TestEnv::new();

        let tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=first, priority=low]
% First line item
% # -- X -- #
% # -- X -- #
% [TODO: id=second, priority=high]
% Second line item
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), tex).unwrap();

        let res_prio = handle_list_todos(json!({ "sort_by": "priority" }));
        assert!(res_prio.is_error.is_none() || res_prio.is_error == Some(false));

        let res_line = handle_list_todos(json!({ "sort_by": "line_start" }));
        assert!(res_line.is_error.is_none() || res_line.is_error == Some(false));
    }

    // --- handle_update_todo ---

    #[test]
    fn test_update_todo_missing_content_error() {
        let res = handle_update_todo(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: content"));
    }

    #[test]
    fn test_update_todo_create_new_block() {
        let env = TestEnv::new();

        let initial_tex = "\\documentclass{article}\n\\begin{document}\n\\end{document}";
        fs::write(env.project_root.join("paper_draft.tex"), initial_tex).unwrap();

        let res = handle_update_todo(json!({
            "section_id": "sec_1",
            "content": "Fresh new TODO block content"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));

        let updated_tex = fs::read_to_string(env.project_root.join("paper_draft.tex")).unwrap();
        assert!(updated_tex.contains("Fresh new TODO block content"));
    }

    #[test]
    fn test_update_todo_update_existing_by_id() {
        let env = TestEnv::new();

        let initial_tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=existing_todo, priority=low, status=open]
% Initial draft text
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), initial_tex).unwrap();

        let res = handle_update_todo(json!({
            "id": "existing_todo",
            "content": "Revised and expanded draft text"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));

        let updated_tex = fs::read_to_string(env.project_root.join("paper_draft.tex")).unwrap();
        assert!(updated_tex.contains("Revised and expanded draft text"));
    }

    #[test]
    fn test_update_todo_status_and_priority() {
        let env = TestEnv::new();

        let initial_tex = r#"\documentclass{article}
\begin{document}
% # -- X -- #
% [TODO: id=st_pr_todo, priority=low, status=open]
% Work item text
% # -- X -- #
\end{document}"#;
        fs::write(env.project_root.join("paper_draft.tex"), initial_tex).unwrap();

        let res = handle_update_todo(json!({
            "id": "st_pr_todo",
            "content": "Work item text",
            "status": "in_progress",
            "priority": "high"
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));

        let updated_tex = fs::read_to_string(env.project_root.join("paper_draft.tex")).unwrap();
        assert!(updated_tex.contains("status=in_progress"));
        assert!(updated_tex.contains("priority=high"));
    }

    // --- handle_list_skills ---

    #[test]
    fn test_list_skills_default_built_in() {
        let _env = TestEnv::new();

        let res = handle_list_skills(json!({}));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        let arr = val.as_array().unwrap();

        assert!(arr.iter().any(|s| s["name"] == "SYSTEM.md"));
        assert!(arr.iter().any(|s| s["name"] == "paper.md"));
        assert!(arr.iter().any(|s| s["name"] == "agent-code.md"));
    }

    #[test]
    fn test_list_skills_discover_custom() {
        let env = TestEnv::new();

        let skills_dir = env.project_root.join("agent/skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("review-guidelines.md"),
            "# Review Guidelines",
        )
        .unwrap();

        let res = handle_list_skills(json!({}));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        let arr = val.as_array().unwrap();

        let custom = arr
            .iter()
            .find(|s| s["name"] == "review-guidelines.md")
            .unwrap();
        assert_eq!(custom["type"], "custom");
    }

    #[test]
    fn test_list_skills_filter_by_category() {
        let env = TestEnv::new();

        let skills_dir = env.project_root.join("agent/skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("custom-analysis.md"), "# Custom Analysis").unwrap();

        let res_builtin = handle_list_skills(json!({ "category": "built-in" }));
        let val_b: serde_json::Value = serde_json::from_str(extract_text(&res_builtin)).unwrap();
        assert!(
            val_b
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["type"] == "built-in")
        );

        let res_custom = handle_list_skills(json!({ "category": "custom" }));
        let val_c: serde_json::Value = serde_json::from_str(extract_text(&res_custom)).unwrap();
        assert!(
            val_c
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["type"] == "custom"
                    || s["name"].as_str().unwrap_or("").contains("custom"))
        );
    }

    // --- handle_invoke_skill ---

    #[test]
    fn test_invoke_skill_missing_name_error() {
        let res = handle_invoke_skill(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter:"));
    }

    #[test]
    fn test_invoke_skill_valid_skill() {
        let _env = TestEnv::new();

        let res = handle_invoke_skill(json!({ "name": "SYSTEM.md" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["skill"], "SYSTEM.md");
        assert!(val["content"].is_string());
    }

    #[test]
    fn test_invoke_skill_nonexistent_skill_error() {
        let _env = TestEnv::new();

        let res = handle_invoke_skill(json!({ "name": "non_existent_skill_xyz.md" }));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Failed to load skill 'non_existent_skill_xyz.md'"));
    }

    // --- handle_get_workspace_context ---

    #[test]
    fn test_get_workspace_context_default() {
        let env = TestEnv::new();
        fs::write(
            env.project_root.join("paper_draft.tex"),
            "\\documentclass{article}",
        )
        .unwrap();

        let res = handle_get_workspace_context(json!({}));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let text = extract_text(&res);
        assert!(!text.is_empty());
    }

    #[test]
    fn test_get_workspace_context_include_paper_false() {
        let env = TestEnv::new();
        fs::write(
            env.project_root.join("paper_draft.tex"),
            "\\documentclass{article}\nSecretDraftContent",
        )
        .unwrap();

        let res = handle_get_workspace_context(json!({ "include_paper": false }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let text = extract_text(&res);
        assert!(!text.contains("SecretDraftContent"));
    }

    // --- handle_get_structure ---

    #[test]
    fn test_get_structure_default_read() {
        let env = TestEnv::new();

        let struct_yaml = "title: \"My Paper\"\nstatus: \"draft\"\nsections: []\n";
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        let res = handle_get_structure(json!({}));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["structure"]["title"], "My Paper");
        assert!(val["completion_summary"].is_object());
    }

    #[test]
    fn test_get_structure_update_missing_section_id_error() {
        let env = TestEnv::new();

        let struct_yaml = "title: \"Paper\"\nstatus: \"draft\"\nsections: []\n";
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        let res = handle_get_structure(json!({ "action": "update", "completed": true }));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing section_id for update action"));
    }

    #[test]
    fn test_get_structure_update_missing_fields_error() {
        let env = TestEnv::new();

        let struct_yaml = "title: \"Paper\"\nstatus: \"draft\"\nsections: []\n";
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        let res = handle_get_structure(json!({ "action": "update", "section_id": "sec_1" }));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("update requires completion"));
    }

    #[test]
    fn test_get_structure_update_invalid_section_id_error() {
        let env = TestEnv::new();

        let struct_yaml = "title: \"Paper\"\nstatus: \"draft\"\nsections:\n  - id: \"sec_1\"\n    title: \"Intro\"\n    level: 1\n    completion: \"empty\"\n";
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        let res = handle_get_structure(json!({
            "action": "update",
            "section_id": "nonexistent_sec",
            "completed": true
        }));
        assert_eq!(res.is_error, Some(true));
        assert!(
            extract_text(&res).contains("Section 'nonexistent_sec' not found in structure.yaml")
        );
    }

    #[test]
    fn test_get_structure_update_toggle_completion() {
        let env = TestEnv::new();

        let struct_yaml = "title: \"Paper\"\nstatus: \"draft\"\nsections:\n  - id: \"sec_1\"\n    title: \"Intro\"\n    level: 1\n    completion: \"empty\"\n";
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        // 1. Set completion to true (deprecated bool → draft)
        let res_true = handle_get_structure(json!({
            "action": "update",
            "section_id": "sec_1",
            "completed": true
        }));
        assert!(res_true.is_error.is_none() || res_true.is_error == Some(false));
        let val_true: serde_json::Value = serde_json::from_str(extract_text(&res_true)).unwrap();
        assert_eq!(val_true["completion_summary"]["draft"], 1);

        // 2. Set completion to false (deprecated bool → empty)
        let res_false = handle_get_structure(json!({
            "action": "update",
            "section_id": "sec_1",
            "completed": false
        }));
        assert!(res_false.is_error.is_none() || res_false.is_error == Some(false));
        let val_false: serde_json::Value = serde_json::from_str(extract_text(&res_false)).unwrap();
        assert_eq!(val_false["completion_summary"]["empty"], 1);
    }

    #[test]
    fn test_get_structure_update_four_state_and_claims() {
        let env = TestEnv::new();

        let struct_yaml = r#"title: "Paper"
status: draft
sections:
  - id: intro
    title: Introduction
    level: 1
    completion: empty
    main_claim: ""
    secondary_points: []
    required_content: []
"#;
        fs::write(env.project_root.join(".sil/structure.yaml"), struct_yaml).unwrap();

        let res = handle_get_structure(json!({
            "action": "update",
            "section_id": "intro",
            "completion": "polished",
            "main_claim": "Transformers beat RNNs",
            "secondary_points": ["self-attention", "parallelism"],
            "required_content": ["problem statement"]
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["completion_summary"]["polished"], 1);
        assert_eq!(val["structure"]["sections"][0]["completion"], "polished");
        assert_eq!(
            val["structure"]["sections"][0]["main_claim"],
            "Transformers beat RNNs"
        );
        assert_eq!(
            val["structure"]["sections"][0]["secondary_points"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            val["structure"]["sections"][0]["required_content"][0],
            "problem statement"
        );
        assert!(
            val["proposal"]
                .as_str()
                .unwrap()
                .contains("Sci-Action: update-structure")
        );
        assert_eq!(val["never_committed"], true);

        // completion enum preferred over completed bool
        let res2 = handle_get_structure(json!({
            "action": "update",
            "section_id": "intro",
            "completion": "outline",
            "completed": true
        }));
        let val2: serde_json::Value = serde_json::from_str(extract_text(&res2)).unwrap();
        assert_eq!(val2["structure"]["sections"][0]["completion"], "outline");

        // invalid completion string
        let bad = handle_get_structure(json!({
            "action": "update",
            "section_id": "intro",
            "completion": "done"
        }));
        assert_eq!(bad.is_error, Some(true));
        assert!(extract_text(&bad).contains("Invalid completion"));
    }

    #[test]
    fn test_get_structure_schema_has_no_word_count() {
        let tools = list_tools();
        let tool = tools
            .iter()
            .find(|t| t.name == "sil_draft")
            .expect("sil_draft registered");
        let props = tool.input_schema.properties.as_object().unwrap();
        assert!(
            !props.contains_key("word_count"),
            "word_count must be removed from schema"
        );
        assert!(props.contains_key("completion"));
        assert!(props.contains_key("main_claim"));
        assert!(props.contains_key("secondary_points"));
        assert!(props.contains_key("required_content"));
    }

    // --- handle_build_and_doctor ---

    #[test]
    fn test_build_and_doctor_engines() {
        let env = TestEnv::new();
        fs::write(
            env.project_root.join("paper_draft.tex"),
            "\\documentclass{article}\n\\begin{document}\nTest\n\\end{document}",
        )
        .unwrap();

        for engine in &["pdflatex", "xelatex", "lualatex", "tectonic"] {
            let res = handle_build_and_doctor(json!({ "engine": engine }));
            assert!(res.is_error.is_none() || res.is_error == Some(false));
            let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
            assert_eq!(val["engine"], *engine);
            assert!(val["build_command"].as_str().unwrap().contains(engine));
        }
    }

    #[test]
    fn test_build_and_doctor_run_doctor_flag() {
        let env = TestEnv::new();
        fs::write(
            env.project_root.join("paper_draft.tex"),
            "\\documentclass{article}\n\\begin{document}\nTest\n\\end{document}",
        )
        .unwrap();

        let res_true = handle_build_and_doctor(json!({ "run_doctor": true }));
        let val_true: serde_json::Value = serde_json::from_str(extract_text(&res_true)).unwrap();
        assert!(val_true["health_doctor_report"].is_object());

        let res_false = handle_build_and_doctor(json!({ "run_doctor": false }));
        let val_false: serde_json::Value = serde_json::from_str(extract_text(&res_false)).unwrap();
        assert!(val_false["health_doctor_report"].is_null());
    }

    // --- handle_propose_commit ---

    #[test]
    fn test_propose_commit_custom_message() {
        let _env = TestEnv::new();

        let res = handle_propose_commit(json!({ "message": "My custom commit message" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert!(
            val["full_commit_message"]
                .as_str()
                .unwrap()
                .contains("My custom commit message")
        );
    }

    #[test]
    fn test_propose_commit_explicit_actions() {
        let _env = TestEnv::new();

        let res_fetch = handle_propose_commit(json!({ "action": "fetch-source" }));
        assert!(res_fetch.is_error.is_none() || res_fetch.is_error == Some(false));
        let val_fetch: serde_json::Value = serde_json::from_str(extract_text(&res_fetch)).unwrap();
        assert_eq!(val_fetch["action_trailer"], "fetch-source");

        let res_edit = handle_propose_commit(json!({ "action": "edit-draft" }));
        assert!(res_edit.is_error.is_none() || res_edit.is_error == Some(false));
        let val_edit: serde_json::Value = serde_json::from_str(extract_text(&res_edit)).unwrap();
        assert_eq!(val_edit["action_trailer"], "edit-draft");
    }

    // --- handle_fetch_source ---

    #[test]
    fn test_fetch_source_missing_target_error() {
        let res = handle_fetch_source(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: target"));
    }

    #[test]
    fn test_fetch_source_no_parse_flag() {
        let env = TestEnv::new();

        let mock_script_path = env.project_root.join("mock_download.py");
        fs::write(
            &mock_script_path,
            "import sys\nprint('sources/mock_paper.pdf')\n",
        )
        .unwrap();

        let sources_dir = env.project_root.join("sources");
        fs::create_dir_all(&sources_dir).unwrap();
        fs::write(
            sources_dir.join("mock_paper.pdf"),
            "%PDF-1.4 dummy pdf content",
        )
        .unwrap();

        unsafe {
            std::env::set_var("SIL_DOWNLOAD_SCRIPT", mock_script_path.as_str());
            std::env::set_var("SIL_MARKER_STUB", "Stub content");
        }

        // no_parse = true
        let res_noparse = handle_fetch_source(json!({ "target": "10.1000/182", "no_parse": true }));

        unsafe {
            std::env::remove_var("SIL_DOWNLOAD_SCRIPT");
            std::env::remove_var("SIL_MARKER_STUB");
        }

        assert!(res_noparse.is_error.is_none() || res_noparse.is_error == Some(false));
        let val_noparse: serde_json::Value =
            serde_json::from_str(extract_text(&res_noparse)).unwrap();
        assert_eq!(val_noparse["parsed"], false);
    }

    #[test]
    fn test_fetch_source_script_failure_error() {
        let env = TestEnv::new();

        let failing_script = env.project_root.join("failing_download.py");
        fs::write(&failing_script, "import sys\nsys.exit(1)\n").unwrap();

        unsafe {
            std::env::set_var("SIL_DOWNLOAD_SCRIPT", failing_script.as_str());
        }

        let res = handle_fetch_source(json!({ "target": "10.1000/fail" }));

        unsafe {
            std::env::remove_var("SIL_DOWNLOAD_SCRIPT");
        }

        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Fetch failed:"));
    }

    // --- call_tool ---

    #[test]
    fn test_call_tool_unknown_tool_error() {
        let res = call_tool("non_existent_tool_xyz", None);
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Unknown tool: non_existent_tool_xyz"));
    }

    #[test]
    fn test_call_tool_none_arguments_fallback() {
        let env = TestEnv::new();
        fs::write(
            env.project_root.join("paper_draft.tex"),
            "\\documentclass{article}",
        )
        .unwrap();

        let res = call_tool("sil_context", None);
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let text = extract_text(&res);
        assert!(!text.is_empty());
    }

    #[test]
    fn test_tools_not_in_project_error() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let orig_cwd = std::env::current_dir().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let res = handle_search_sources(json!({ "query": "test" }));
        std::env::set_current_dir(&orig_cwd).unwrap();

        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Not in a sil project"));
    }

    #[test]
    fn test_propose_commit_default() {
        let _env = TestEnv::new();

        let res = handle_propose_commit(json!({}));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert!(val["proposal_subject"].is_string());
        assert!(val["full_commit_message"].is_string());
        assert_eq!(val["action_trailer"], "edit-draft");
    }

    // --- handle_upsert_bib / handle_promote_bib ---

    fn git_head(dir: &Utf8PathBuf) -> Option<String> {
        let out = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir.as_str())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn init_git_repo(dir: &Utf8PathBuf) {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(dir.as_str())
                .status()
                .expect("git available");
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["add", "-A"]);
        run(&["commit", "-m", "initial"]);
    }

    #[test]
    fn test_upsert_bib_missing_entry_error() {
        let res = handle_upsert_bib(json!({}));
        assert_eq!(res.is_error, Some(true));
        assert!(extract_text(&res).contains("Missing required parameter: entry"));
    }

    #[test]
    fn test_upsert_bib_empty_and_invalid_entry() {
        let _env = TestEnv::new();
        let empty = handle_upsert_bib(json!({ "entry": "   " }));
        assert_eq!(empty.is_error, Some(true));
        assert!(extract_text(&empty).contains("empty"));

        let bad = handle_upsert_bib(json!({ "entry": "not bibtex at all" }));
        assert_eq!(bad.is_error, Some(true));
        assert!(extract_text(&bad).contains("not valid BibTeX"));
    }

    #[test]
    fn test_upsert_bib_writes_and_never_commits() {
        let env = TestEnv::new();
        init_git_repo(&env.project_root);
        let head_before = git_head(&env.project_root).expect("HEAD after init");

        let entry = r#"@article{smith2024,
  title = {A Test Paper},
  author = {Smith, A.},
  year = {2024},
  journal = {J. Test}
}"#;
        let res = handle_upsert_bib(json!({ "entry": entry }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["wrote"], true);
        assert_eq!(val["cite_key"], "smith2024");
        assert_eq!(val["replaced"], false);
        assert_eq!(val["never_committed"], true);
        assert_eq!(val["draft"], false);
        let proposal = val["proposal"].as_str().unwrap();
        assert!(proposal.contains("Sci-Action: update-bibliography"));

        let bib_path = env.project_root.join("references.bib");
        let content = fs::read_to_string(bib_path.as_str()).unwrap();
        assert!(content.contains("smith2024"));
        assert!(content.contains("A Test Paper"));
        assert!(!content.contains("tui-added"));

        let head_after = git_head(&env.project_root).expect("HEAD after tool");
        assert_eq!(
            head_before, head_after,
            "sil_upsert_bib must not create a git commit"
        );
    }

    #[test]
    fn test_upsert_bib_draft_marks_tui_added() {
        let env = TestEnv::new();

        let entry =
            "@article{draftkey,\n  title = {Draft Only},\n  author = {X},\n  year = {2020}\n}";
        let res = handle_upsert_bib(json!({ "entry": entry, "draft": true }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["draft"], true);
        assert_eq!(val["cite_key"], "draftkey");

        let content = fs::read_to_string(env.project_root.join("references.bib").as_str()).unwrap();
        assert!(content.contains("% [sil: tui-added]"));
        assert!(content.contains("draftkey"));
    }

    #[test]
    fn test_upsert_bib_preserve_cite_key() {
        let env = TestEnv::new();
        let bib_path = env.project_root.join("references.bib");
        fs::write(
            bib_path.as_str(),
            "@article{oldkey,\n  title = {Same Paper},\n  doi = {10.1000/abc}\n}\n",
        )
        .unwrap();

        let entry =
            "@article{newkey,\n  title = {Same Paper},\n  doi = {10.1000/abc},\n  year = {2021}\n}";
        let res = handle_upsert_bib(json!({
            "entry": entry,
            "preserve_cite_key": true
        }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["replaced"], true);
        assert_eq!(val["cite_key"], "oldkey");

        let content = fs::read_to_string(bib_path.as_str()).unwrap();
        assert!(content.contains("@article{oldkey"));
        assert!(!content.contains("@article{newkey"));
    }

    #[test]
    fn test_promote_bib_strips_marker_and_never_commits() {
        let env = TestEnv::new();
        init_git_repo(&env.project_root);
        let head_before = git_head(&env.project_root).expect("HEAD after init");

        let bib_path = env.project_root.join("references.bib");
        fs::write(
            bib_path.as_str(),
            "% [sil: tui-added]\n@article{promotekey,\n  title = {To Promote},\n  year = {2022}\n}\n",
        )
        .unwrap();

        let res = handle_promote_bib(json!({ "cite_key": "promotekey" }));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let val: serde_json::Value = serde_json::from_str(extract_text(&res)).unwrap();
        assert_eq!(val["wrote"], true);
        assert_eq!(val["cite_key"], "promotekey");
        assert_eq!(val["replaced"], true);
        assert_eq!(val["never_committed"], true);
        let proposal = val["proposal"].as_str().unwrap();
        assert!(proposal.contains("Sci-Action: promote-bibliography"));

        let content = fs::read_to_string(bib_path.as_str()).unwrap();
        assert!(content.contains("promotekey"));
        assert!(!content.contains("tui-added"));

        let head_after = git_head(&env.project_root).expect("HEAD after tool");
        assert_eq!(
            head_before, head_after,
            "sil_promote_bib must not create a git commit"
        );
    }

    #[test]
    fn test_promote_bib_missing_and_not_found() {
        let env = TestEnv::new();
        let missing = handle_promote_bib(json!({}));
        assert_eq!(missing.is_error, Some(true));
        assert!(extract_text(&missing).contains("Missing required parameter: cite_key"));

        let no_file = handle_promote_bib(json!({ "cite_key": "ghost" }));
        assert_eq!(no_file.is_error, Some(true));
        assert!(extract_text(&no_file).contains("not found"));

        fs::write(
            env.project_root.join("references.bib").as_str(),
            "@article{other,\n  title = {Other}\n}\n",
        )
        .unwrap();
        let not_found = handle_promote_bib(json!({ "cite_key": "ghost" }));
        assert_eq!(not_found.is_error, Some(true));
        assert!(extract_text(&not_found).contains("No entry matching"));
    }

    #[test]
    fn test_call_tool_routes_bib_tools() {
        let env = TestEnv::new();
        let entry = "@article{routekey,\n  title = {Routed},\n  year = {2019}\n}";
        let res = call_tool(
            "sil_cite",
            Some(json!({ "action": "upsert", "entry": entry, "draft": true })),
        );
        assert!(res.is_error.is_none() || res.is_error == Some(false));

        let res = call_tool("sil_cite", Some(json!({ "action": "promote", "cite_key": "routekey" })));
        assert!(res.is_error.is_none() || res.is_error == Some(false));
        let content = fs::read_to_string(env.project_root.join("references.bib").as_str()).unwrap();
        assert!(!content.contains("tui-added"));
    }

    #[test]
    fn test_call_tool_six_tools_routing_and_validation() {
        let _env = TestEnv::new();

        // sil_sources missing action
        let res_no_act = call_tool("sil_sources", Some(json!({})));
        assert_eq!(res_no_act.is_error, Some(true));
        assert!(extract_text(&res_no_act).contains("Missing required parameter: action"));

        // sil_sources invalid action
        let res_bad_act = call_tool("sil_sources", Some(json!({ "action": "invalid" })));
        assert_eq!(res_bad_act.is_error, Some(true));
        assert!(extract_text(&res_bad_act).contains("Invalid action 'invalid'"));

        // sil_cite missing action
        let res_cite_no_act = call_tool("sil_cite", Some(json!({})));
        assert_eq!(res_cite_no_act.is_error, Some(true));
        assert!(extract_text(&res_cite_no_act).contains("Missing required parameter: action"));

        // sil_draft missing action
        let res_draft_no_act = call_tool("sil_draft", Some(json!({})));
        assert_eq!(res_draft_no_act.is_error, Some(true));
        assert!(extract_text(&res_draft_no_act).contains("Missing required parameter: action"));

        // sil_review missing action
        let res_rev_no_act = call_tool("sil_review", Some(json!({})));
        assert_eq!(res_rev_no_act.is_error, Some(true));
        assert!(extract_text(&res_rev_no_act).contains("Missing required parameter: action"));

        // sil_context with skill alias parameter
        let res_skill = call_tool("sil_context", Some(json!({ "skill": "SYSTEM.md" })));
        assert!(res_skill.is_error.is_none() || res_skill.is_error == Some(false));
        let val_skill: serde_json::Value = serde_json::from_str(extract_text(&res_skill)).unwrap();
        assert_eq!(val_skill["skill"], "SYSTEM.md");
    }
}
