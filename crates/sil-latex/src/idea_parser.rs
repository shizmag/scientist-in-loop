//! Parser for `# -- X -- #` Idea and TODO blocks in LaTeX documents.

#![allow(clippy::collapsible_if)]

use sil_core::IdeaBlock;

/// Parse `# -- X -- #` idea and TODO blocks from LaTeX file contents.
///
/// Supports markers like:
/// ```text
/// # -- X -- #
/// [TODO: id=todo-1, priority=high, status=open, author=human, tags=ablation]
/// Idea text here
/// # -- X -- #
/// ```
/// or commented markers:
/// ```text
/// % # -- X -- #
/// % [TODO: id=todo-1, priority=high, status=open, author=human, tags=ablation]
/// % TODO: update figures
/// % # -- X -- #
/// ```
pub fn parse_idea_blocks(tex_content: &str) -> Vec<IdeaBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_content = Vec::new();
    let mut start_line = 0;
    let mut current_section: Option<String> = None;
    let mut block_counter = 1;

    for (idx, line) in tex_content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        // Track current section title if encountered
        if trimmed.starts_with("\\section") || trimmed.starts_with("\\subsection") {
            let s_opt = trimmed.find('{');
            let e_opt = trimmed.rfind('}');
            if let (Some(s), Some(e)) = (s_opt, e_opt) {
                if s < e {
                    current_section = Some(trimmed[s + 1..e].trim().to_string());
                }
            }
        }

        // Check for boundary marker: contains "# -- X -- #" or "#-- X --#"
        if trimmed.contains("# -- X -- #") || trimmed.contains("#-- X --#") {
            if !in_block {
                in_block = true;
                start_line = line_num;
                current_content.clear();
            } else {
                // End of block
                in_block = false;
                let end_line = line_num;

                // Clean leading LaTeX comment chars '%' if present across lines
                let cleaned_lines: Vec<&str> = current_content
                    .iter()
                    .map(|l: &&str| (*l).trim().strip_prefix('%').unwrap_or(*l).trim())
                    .collect();

                let mut id = None;
                let mut status = "open".to_string();
                let mut priority = "medium".to_string();
                let mut author_type = "human".to_string();
                let mut tags = Vec::new();

                let header_idx = cleaned_lines
                    .iter()
                    .position(|l| l.starts_with("[TODO:") || l.starts_with("[TODO "));

                let body_lines: Vec<&str> = if let Some(h_idx) = header_idx {
                    let header_line = cleaned_lines[h_idx];
                    let inner = if let Some(start) = header_line.find("[TODO:") {
                        &header_line[start + 6..]
                    } else if let Some(start) = header_line.find("[TODO ") {
                        &header_line[start + 6..]
                    } else {
                        header_line
                    };
                    let inner = inner.trim_end_matches(']').trim();

                    parse_header_kv(
                        inner,
                        &mut id,
                        &mut status,
                        &mut priority,
                        &mut author_type,
                        &mut tags,
                    );

                    cleaned_lines
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != h_idx)
                        .map(|(_, &l)| l)
                        .collect()
                } else {
                    cleaned_lines.clone()
                };

                let content = body_lines.join("\n").trim().to_string();

                if !content.is_empty() {
                    let final_id = id.unwrap_or_else(|| format!("idea_{block_counter}"));
                    block_counter += 1;

                    let mut block = IdeaBlock::new(
                        final_id,
                        content,
                        current_section.clone(),
                        start_line,
                        end_line,
                    );
                    block.status = status;
                    block.priority = priority;
                    block.author_type = author_type;
                    block.tags = tags;
                    blocks.push(block);
                }
            }
        } else if in_block {
            current_content.push(line);
        }
    }

    blocks
}

/// Helper to parse key-value options inside `[TODO: ...]` header.
fn parse_header_kv(
    inner: &str,
    out_id: &mut Option<String>,
    out_status: &mut String,
    out_priority: &mut String,
    out_author: &mut String,
    out_tags: &mut Vec<String>,
) {
    let mut matches = Vec::new();
    let bytes = inner.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'=' && i > 0 {
            let mut key_start = i;
            while key_start > 0 {
                let prev = bytes[key_start - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' {
                    key_start -= 1;
                } else {
                    break;
                }
            }
            if key_start < i {
                let key = &inner[key_start..i].trim();
                matches.push((key_start, i, key.to_string()));
            }
        }
        i += 1;
    }

    for idx in 0..matches.len() {
        let (_k_start, eq_idx, key) = &matches[idx];
        let val_start = eq_idx + 1;
        let val_end = if idx + 1 < matches.len() {
            matches[idx + 1].0
        } else {
            inner.len()
        };
        let raw_val = inner[val_start..val_end]
            .trim()
            .trim_matches(',')
            .trim_matches(';')
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim();

        match key.to_lowercase().as_str() {
            "id" => {
                if !raw_val.is_empty() {
                    *out_id = Some(raw_val.to_string());
                }
            }
            "status" => {
                if !raw_val.is_empty() {
                    *out_status = raw_val.to_string();
                }
            }
            "priority" | "prio" => {
                if !raw_val.is_empty() {
                    *out_priority = raw_val.to_string();
                }
            }
            "author" | "author_type" => {
                if !raw_val.is_empty() {
                    *out_author = raw_val.to_string();
                }
            }
            "tags" | "tag" if !raw_val.is_empty() => {
                *out_tags = raw_val
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            _ => {}
        }
    }
}

/// Format an `IdeaBlock` into a `% # -- X -- #` (or `# -- X -- #`) string block.
pub fn format_idea_block(block: &IdeaBlock, commented: bool) -> String {
    let prefix = if commented { "% " } else { "" };
    let mut header_opts = vec![format!("id={}", block.id)];
    header_opts.push(format!("priority={}", block.priority));
    header_opts.push(format!("status={}", block.status));
    header_opts.push(format!("author={}", block.author_type));
    if !block.tags.is_empty() {
        header_opts.push(format!("tags={}", block.tags.join(",")));
    }
    let header_line = format!("[TODO: {}]", header_opts.join(", "));

    let mut lines = Vec::new();
    lines.push(format!("{}# -- X -- #", prefix));
    lines.push(format!("{}{}", prefix, header_line));
    for line in block.content.lines() {
        lines.push(format!("{}{}", prefix, line));
    }
    lines.push(format!("{}# -- X -- #", prefix));
    lines.join("\n")
}

/// Update an existing `# -- X -- #` block matching `block.id` in `tex_content`,
/// or insert it if no block with `block.id` exists.
pub fn update_or_insert_idea_block(tex_content: &str, block: &IdeaBlock) -> String {
    let existing_blocks = parse_idea_blocks(tex_content);
    if let Some(target) = existing_blocks.iter().find(|b| b.id == block.id) {
        let lines: Vec<&str> = tex_content.lines().collect();
        let mut new_lines = Vec::new();

        let start_idx = target.line_start.saturating_sub(1);
        let end_idx = target.line_end.saturating_sub(1);

        for (idx, line) in lines.iter().enumerate() {
            if idx == start_idx {
                let commented = line.trim().starts_with('%');
                new_lines.push(format_idea_block(block, commented));
            } else if idx > start_idx && idx <= end_idx {
                continue;
            } else {
                new_lines.push(line.to_string());
            }
        }
        let mut result = new_lines.join("\n");
        if tex_content.ends_with('\n') && !result.ends_with('\n') {
            result.push('\n');
        }
        result
    } else {
        let formatted = format_idea_block(block, true);
        let lines: Vec<&str> = tex_content.lines().collect();

        if let Some(sec_target) = &block.section_id {
            let mut insert_idx = None;
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if (trimmed.starts_with("\\section") || trimmed.starts_with("\\subsection"))
                    && trimmed.contains(sec_target)
                {
                    insert_idx = Some(idx + 1);
                    break;
                }
            }
            if let Some(idx) = insert_idx {
                let mut new_lines = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    new_lines.push(line.to_string());
                    if i + 1 == idx {
                        new_lines.push(String::new());
                        new_lines.push(formatted.clone());
                    }
                }
                let mut result = new_lines.join("\n");
                if tex_content.ends_with('\n') && !result.ends_with('\n') {
                    result.push('\n');
                }
                return result;
            }
        }

        let mut result = tex_content.to_string();
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result.push('\n');
        result.push_str(&formatted);
        result.push('\n');
        result
    }
}

/// Helper to update status of a block by ID in LaTeX content string cleanly.
pub fn update_idea_block_status(tex_content: &str, block_id: &str, new_status: &str) -> String {
    let existing_blocks = parse_idea_blocks(tex_content);
    if let Some(mut target) = existing_blocks.into_iter().find(|b| b.id == block_id) {
        target.status = new_status.to_string();
        update_or_insert_idea_block(tex_content, &target)
    } else {
        tex_content.to_string()
    }
}

/// Strip all `# -- X -- #` (or `#-- X --#`) idea and TODO blocks from LaTeX file contents.
///
/// Lines between opening and closing markers (inclusive) are removed.
pub fn strip_idea_blocks(tex_content: &str) -> String {
    let mut kept_lines = Vec::new();
    let mut in_block = false;

    for line in tex_content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("# -- X -- #") || trimmed.contains("#-- X --#") {
            in_block = !in_block;
            continue;
        }
        if !in_block {
            kept_lines.push(line);
        }
    }

    let mut result = kept_lines.join("\n");
    if tex_content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_commented_idea_block() {
        let tex = r#"
\section{Introduction}
Some text here.

% # -- X -- #
% Idea: Add an ablation table comparing model A vs model B.
% TODO: Verify equation 3 derivation.
% # -- X -- #

More text.
"#;
        let blocks = parse_idea_blocks(tex);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].section_id.as_deref(), Some("Introduction"));
        assert!(blocks[0].content.contains("ablation table"));
        assert!(blocks[0].content.contains("Verify equation 3"));
        assert_eq!(blocks[0].status, "open");
        assert_eq!(blocks[0].priority, "medium");
        assert_eq!(blocks[0].author_type, "human");
    }

    #[test]
    fn parse_structured_metadata_header() {
        let tex = r#"
\section{Experiments}
% # -- X -- #
% [TODO: id=todo-101, priority=critical, status=in_progress, author=agent, tags=ablation,baseline]
% Perform hyperparameter grid search for learning rate.
% # -- X -- #
"#;
        let blocks = parse_idea_blocks(tex);
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.id, "todo-101");
        assert_eq!(b.priority, "critical");
        assert_eq!(b.status, "in_progress");
        assert_eq!(b.author_type, "agent");
        assert_eq!(b.tags, vec!["ablation", "baseline"]);
        assert_eq!(b.content, "Perform hyperparameter grid search for learning rate.");
        assert_eq!(b.section_id.as_deref(), Some("Experiments"));
    }

    #[test]
    fn update_and_insert_idea_blocks_in_latex() {
        let tex = r#"\section{Introduction}
Initial text.

% # -- X -- #
% [TODO: id=todo-1, priority=high, status=open, author=human, tags=review]
% Check related work citations.
% # -- X -- #
"#;
        // 1. Update status
        let updated = update_idea_block_status(tex, "todo-1", "resolved");
        let blocks = parse_idea_blocks(&updated);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "todo-1");
        assert_eq!(blocks[0].status, "resolved");

        // 2. Insert new block into Introduction section
        let mut new_block = IdeaBlock::new("todo-2", "Add motivation figure", Some("Introduction".into()), 0, 0);
        new_block.priority = "critical".into();
        new_block.tags = vec!["figure".into(), "intro".into()];

        let inserted = update_or_insert_idea_block(&updated, &new_block);
        let blocks2 = parse_idea_blocks(&inserted);
        assert_eq!(blocks2.len(), 2);
        let b2 = blocks2.iter().find(|b| b.id == "todo-2").expect("todo-2 block should exist");
        assert_eq!(b2.priority, "critical");
        assert_eq!(b2.tags, vec!["figure", "intro"]);
    }

    #[test]
    fn parse_uncommented_idea_block() {
        let tex = r#"
# -- X -- #
Need to revise conclusion.
# -- X -- #
"#;
        let blocks = parse_idea_blocks(tex);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Need to revise conclusion.");
    }

    #[test]
    fn parse_multiple_blocks_across_sections() {
        let tex = r#"
\section{Background}
% # -- X -- #
% Note 1 in Background
% # -- X -- #

\subsection{Related Work}
% # -- X -- #
% Note 2 in Related Work
% # -- X -- #
"#;
        let blocks = parse_idea_blocks(tex);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].section_id.as_deref(), Some("Background"));
        assert_eq!(blocks[0].content, "Note 1 in Background");
        assert_eq!(blocks[1].section_id.as_deref(), Some("Related Work"));
        assert_eq!(blocks[1].content, "Note 2 in Related Work");
    }

    #[test]
    fn parse_empty_block_ignored() {
        let tex = r#"
# -- X -- #

# -- X -- #
"#;
        let blocks = parse_idea_blocks(tex);
        assert!(blocks.is_empty());
    }

    #[test]
    fn parse_special_characters_and_latex_code() {
        let tex = r#"
\section{Methods}
% # -- X -- #
% TODO: Check $\alpha_i + \beta_j = 1$ derivation & \cite{Smith2024}.
% # -- X -- #
"#;
        let blocks = parse_idea_blocks(tex);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].content.contains(r"$\alpha_i + \beta_j = 1$"));
        assert!(blocks[0].content.contains(r"\cite{Smith2024}"));
    }

    #[test]
    fn parse_no_section_defaults_to_none() {
        let tex = r#"
% # -- X -- #
% General preamble idea before any section.
% # -- X -- #
"#;
        let blocks = parse_idea_blocks(tex);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].section_id, None);
        assert_eq!(blocks[0].content, "General preamble idea before any section.");
    }

    #[test]
    fn test_strip_idea_blocks() {
        let tex = r#"\section{Introduction}
Some text here.

% # -- X -- #
% Idea: Add an ablation table comparing model A vs model B.
% TODO: Verify equation 3 derivation.
% # -- X -- #

More text here."#;
        let stripped = strip_idea_blocks(tex);
        assert!(!stripped.contains("ablation table"));
        assert!(!stripped.contains("# -- X -- #"));
        assert!(stripped.contains("Some text here."));
        assert!(stripped.contains("More text here."));
    }
}


