//! Parser for `# -- X -- #` Idea and TODO blocks in LaTeX documents.

#![allow(clippy::collapsible_if)]

use sil_core::IdeaBlock;


/// Parse `# -- X -- #` idea and TODO blocks from LaTeX file contents.
///
/// Supports markers like:
/// ```text
/// # -- X -- #
/// Idea text here
/// # -- X -- #
/// ```
/// or commented markers:
/// ```text
/// % # -- X -- #
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



        // Check for boundary marker: contains "# -- X -- #"
        if trimmed.contains("# -- X -- #") {
            if !in_block {
                in_block = true;
                start_line = line_num;
                current_content.clear();
            } else {
                // End of block
                in_block = false;
                let end_line = line_num;
                let raw_text = current_content.join("\n");
                // Clean leading LaTeX comment chars '%' if present across lines
                let cleaned_lines: Vec<&str> = raw_text
                    .lines()
                    .map(|l| l.trim().strip_prefix('%').unwrap_or(l).trim())
                    .collect();
                let content = cleaned_lines.join("\n").trim().to_string();

                if !content.is_empty() {
                    let id = format!("idea_{block_counter}");
                    block_counter += 1;
                    blocks.push(IdeaBlock::new(
                        id,
                        content,
                        current_section.clone(),
                        start_line,
                        end_line,
                    ));
                }
            }
        } else if in_block {
            current_content.push(line);
        }
    }

    blocks
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
}
