//! Domain types for # -- X -- # Idea and TODO blocks in paper draft prose.

use serde::{Deserialize, Serialize};

/// Represents an idea or TODO block extracted from `paper_draft.tex`
/// bounded by `# -- X -- #` (or `% # -- X -- #`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdeaBlock {
    /// Unique identifier or index for the block.
    pub id: String,
    /// Extract text content inside the block.
    pub content: String,
    /// Section ID or title where the block was found.
    pub section_id: Option<String>,
    /// Starting line number in the source file.
    pub line_start: usize,
    /// Ending line number in the source file.
    pub line_end: usize,
    /// Status: "open" | "in_progress" | "resolved" | "deferred" (default "open").
    pub status: String,
    /// Priority: "low" | "medium" | "high" | "critical" (default "medium").
    pub priority: String,
    /// Author type: "human" | "agent" (default "human").
    pub author_type: String,
    /// Categorization tags.
    pub tags: Vec<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Type alias for `IdeaBlock`.
pub type TodoIdea = IdeaBlock;

impl IdeaBlock {
    /// Create a new `IdeaBlock` with default metadata.
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        section_id: Option<String>,
        line_start: usize,
        line_end: usize,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            section_id,
            line_start,
            line_end,
            status: "open".to_string(),
            priority: "medium".to_string(),
            author_type: "human".to_string(),
            tags: Vec::new(),
            created_at: String::new(),
        }
    }
}

