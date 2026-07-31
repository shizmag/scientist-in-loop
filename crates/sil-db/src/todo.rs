//! Database operations for TODO blocks and ideas.

use rusqlite::Connection;
use sil_core::IdeaBlock;

use crate::error::DbError;

/// Insert a single TodoIdea into the database.
pub fn insert_todo_idea(conn: &Connection, idea: &IdeaBlock) -> Result<(), DbError> {
    let tags_str = idea.tags.join(",");
    conn.execute(
        "INSERT INTO todo_ideas (id, content, section_id, line_start, line_end, status, priority, author_type, tags, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, COALESCE(NULLIF(?10, ''), datetime('now')))",
        rusqlite::params![
            idea.id,
            idea.content,
            idea.section_id,
            idea.line_start as i64,
            idea.line_end as i64,
            idea.status,
            idea.priority,
            idea.author_type,
            tags_str,
            idea.created_at,
        ],
    )?;
    Ok(())
}

/// Update an existing TodoIdea in the database.
pub fn update_todo_idea(conn: &Connection, idea: &IdeaBlock) -> Result<(), DbError> {
    let tags_str = idea.tags.join(",");
    conn.execute(
        "UPDATE todo_ideas SET content=?2, section_id=?3, line_start=?4, line_end=?5, status=?6, priority=?7, author_type=?8, tags=?9 WHERE id=?1",
        rusqlite::params![
            idea.id,
            idea.content,
            idea.section_id,
            idea.line_start as i64,
            idea.line_end as i64,
            idea.status,
            idea.priority,
            idea.author_type,
            tags_str,
        ],
    )?;
    Ok(())
}

/// Upsert a TodoIdea (insert or update on conflict id) in the database.
pub fn upsert_todo_idea(conn: &Connection, idea: &IdeaBlock) -> Result<(), DbError> {
    let tags_str = idea.tags.join(",");
    conn.execute(
        "INSERT INTO todo_ideas (id, content, section_id, line_start, line_end, status, priority, author_type, tags, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, COALESCE(NULLIF(?10, ''), datetime('now')))
         ON CONFLICT(id) DO UPDATE SET
            content=excluded.content,
            section_id=excluded.section_id,
            line_start=excluded.line_start,
            line_end=excluded.line_end,
            status=excluded.status,
            priority=excluded.priority,
            author_type=excluded.author_type,
            tags=excluded.tags",
        rusqlite::params![
            idea.id,
            idea.content,
            idea.section_id,
            idea.line_start as i64,
            idea.line_end as i64,
            idea.status,
            idea.priority,
            idea.author_type,
            tags_str,
            idea.created_at,
        ],
    )?;
    Ok(())
}

/// Delete a TodoIdea by id. Returns true if a row was deleted.
pub fn delete_todo_idea(conn: &Connection, id: &str) -> Result<bool, DbError> {
    let count = conn.execute("DELETE FROM todo_ideas WHERE id = ?", [id])?;
    Ok(count > 0)
}

/// Get a single TodoIdea by id.
pub fn get_todo_idea_by_id(conn: &Connection, id: &str) -> Result<Option<IdeaBlock>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, content, section_id, line_start, line_end, status, priority, author_type, tags, created_at FROM todo_ideas WHERE id = ?",
    )?;
    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        let tags_raw: String = row.get(8)?;
        let tags = tags_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        Ok(Some(IdeaBlock {
            id: row.get(0)?,
            content: row.get(1)?,
            section_id: row.get(2)?,
            line_start: row.get::<_, i64>(3)? as usize,
            line_end: row.get::<_, i64>(4)? as usize,
            status: row.get(5)?,
            priority: row.get(6)?,
            author_type: row.get(7)?,
            tags,
            created_at: row.get(9)?,
        }))
    } else {
        Ok(None)
    }
}

/// Replace all idea/TODO blocks in database with fresh set.
pub fn replace_todo_ideas(conn: &Connection, ideas: &[IdeaBlock]) -> Result<(), DbError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM todo_ideas", [])?;
    let mut stmt = tx.prepare(
        "INSERT INTO todo_ideas (id, content, section_id, line_start, line_end, status, priority, author_type, tags, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, COALESCE(NULLIF(?10, ''), datetime('now')))",
    )?;
    for idea in ideas {
        let tags_str = idea.tags.join(",");
        stmt.execute(rusqlite::params![
            idea.id,
            idea.content,
            idea.section_id,
            idea.line_start as i64,
            idea.line_end as i64,
            idea.status,
            idea.priority,
            idea.author_type,
            tags_str,
            idea.created_at,
        ])?;
    }
    drop(stmt);
    tx.commit()?;
    Ok(())
}

/// List todo_ideas with optional filters and sorting.
pub fn list_todo_ideas_filtered(
    conn: &Connection,
    status: Option<&str>,
    priority: Option<&str>,
    section_id: Option<&str>,
    sort_by: Option<&str>,
) -> Result<Vec<IdeaBlock>, DbError> {
    let mut query = String::from(
        "SELECT id, content, section_id, line_start, line_end, status, priority, author_type, tags, created_at FROM todo_ideas WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(s) = status {
        query.push_str(" AND status = ?");
        params.push(Box::new(s.to_string()));
    }
    if let Some(p) = priority {
        query.push_str(" AND priority = ?");
        params.push(Box::new(p.to_string()));
    }
    if let Some(sec) = section_id {
        query.push_str(" AND section_id = ?");
        params.push(Box::new(sec.to_string()));
    }

    match sort_by {
        Some("priority") => {
            query.push_str(" ORDER BY CASE priority WHEN 'critical' THEN 1 WHEN 'high' THEN 2 WHEN 'medium' THEN 3 WHEN 'low' THEN 4 ELSE 5 END ASC, line_start ASC");
        }
        Some("date") => {
            query.push_str(" ORDER BY created_at DESC, line_start ASC");
        }
        Some("section") => {
            query.push_str(" ORDER BY section_id ASC, line_start ASC");
        }
        _ => {
            query.push_str(" ORDER BY line_start ASC");
        }
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|p| p.as_ref() as &dyn rusqlite::ToSql)
        .collect();
    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let tags_raw: String = row.get(8)?;
        let tags = tags_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        Ok(IdeaBlock {
            id: row.get(0)?,
            content: row.get(1)?,
            section_id: row.get(2)?,
            line_start: row.get::<_, i64>(3)? as usize,
            line_end: row.get::<_, i64>(4)? as usize,
            status: row.get(5)?,
            priority: row.get(6)?,
            author_type: row.get(7)?,
            tags,
            created_at: row.get(9)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

    #[test]
    fn test_todo_ideas_direct_functions_and_sorting() {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();

        // 1. insert_todo_idea
        let mut idea1 = IdeaBlock::new("todo_1", "Fix bug in parser", Some("sec_b".into()), 10, 20);
        idea1.status = "open".into();
        idea1.priority = "high".into();
        idea1.author_type = "human".into();
        idea1.created_at = "2026-01-01 10:00:00".into();
        insert_todo_idea(&conn, &idea1).unwrap();

        let mut idea2 = IdeaBlock::new("todo_2", "Refactor cache", Some("sec_a".into()), 5, 15);
        idea2.status = "open".into();
        idea2.priority = "low".into();
        idea2.author_type = "agent".into();
        idea2.created_at = "2026-01-02 10:00:00".into();
        insert_todo_idea(&conn, &idea2).unwrap();

        // 2. get_todo_idea_by_id existing & non-existing
        let fetched1 = get_todo_idea_by_id(&conn, "todo_1").unwrap();
        assert!(fetched1.is_some());
        assert_eq!(fetched1.unwrap().content, "Fix bug in parser");

        let missing = get_todo_idea_by_id(&conn, "todo_missing").unwrap();
        assert!(missing.is_none());

        // 3. update_todo_idea
        let mut idea1_updated = idea1.clone();
        idea1_updated.content = "Fix bug in parser immediately".into();
        update_todo_idea(&conn, &idea1_updated).unwrap();
        let fetched1_updated = get_todo_idea_by_id(&conn, "todo_1").unwrap().unwrap();
        assert_eq!(fetched1_updated.content, "Fix bug in parser immediately");

        // 4. upsert_todo_idea
        let mut idea3 = IdeaBlock::new("todo_3", "New feature", Some("sec_c".into()), 30, 40);
        idea3.status = "closed".into();
        idea3.priority = "critical".into();
        idea3.created_at = "2026-01-03 10:00:00".into();
        upsert_todo_idea(&conn, &idea3).unwrap();


        // 5. list_todo_ideas_filtered with sort_by = "date", "section", and unknown
        let sort_date = list_todo_ideas_filtered(&conn, None, None, None, Some("date")).unwrap();
        assert_eq!(sort_date.len(), 3);
        assert_eq!(sort_date[0].id, "todo_3"); // newest first

        let sort_sec = list_todo_ideas_filtered(&conn, None, None, None, Some("section")).unwrap();
        assert_eq!(sort_sec.len(), 3);
        assert_eq!(sort_sec[0].id, "todo_2"); // sec_a comes before sec_b

        let sort_other = list_todo_ideas_filtered(&conn, None, None, None, Some("other")).unwrap();
        assert_eq!(sort_other.len(), 3);

        // Filter all criteria simultaneously
        let filtered = list_todo_ideas_filtered(&conn, Some("open"), Some("high"), Some("sec_b"), None).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "todo_1");

        // 6. delete_todo_idea existing & non-existing
        assert!(delete_todo_idea(&conn, "todo_1").unwrap());
        assert!(!delete_todo_idea(&conn, "todo_1").unwrap());
    }
}

