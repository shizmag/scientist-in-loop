//! Undo journal for tracking and reverting TUI file mutations.
//!
//! Maintains capped generation snapshots under `<project_root>/.sil/undo/`
//! without invoking `git` operations.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::SilError;

/// Snapshot of a single file's contents prior to a mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoFileSnapshot {
    /// File path relative to project root (or absolute if outside project).
    pub path: Utf8PathBuf,
    /// Exact byte content of the file before mutation.
    pub content: Vec<u8>,
}

/// A single generation record containing one or more file snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoGeneration {
    /// Monotonically increasing generation identifier.
    pub id: usize,
    /// UNIX timestamp in seconds when the snapshot was recorded.
    pub timestamp: u64,
    /// Human-readable operation name or description.
    pub op: String,
    /// Collection of file snapshots captured for this generation.
    pub files: Vec<UndoFileSnapshot>,
}

/// Journal managing capped undo generations in `<project_root>/.sil/undo/`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UndoJournal;

impl UndoJournal {
    /// Maximum number of undo generations retained on disk (oldest pruned when exceeded).
    pub const MAX_GENERATIONS: usize = 10;

    /// Create a snapshot of the specified files under `<project_root>/.sil/undo/`.
    ///
    /// If a file does not exist on disk, an empty byte snapshot is recorded.
    /// File paths are stored relative to `project_root` whenever possible.
    ///
    /// Returns the assigned generation ID.
    pub fn snapshot(
        project_root: &Utf8Path,
        op: impl Into<String>,
        files: &[Utf8PathBuf],
    ) -> Result<usize, SilError> {
        let undo_dir = project_root.join(crate::paths::rel::UNDO);
        fs::create_dir_all(undo_dir.as_std_path())?;

        let mut existing = Self::list_generations(&undo_dir)?;
        let next_id = existing.last().map(|(id, _)| *id + 1).unwrap_or(1);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut snapshots = Vec::with_capacity(files.len());
        for file_path in files {
            let (abs_path, rel_path) = if file_path.is_absolute() {
                let rel = match file_path.strip_prefix(project_root) {
                    Ok(r) => r.to_path_buf(),
                    Err(_) => file_path.clone(),
                };
                (file_path.clone(), rel)
            } else {
                (project_root.join(file_path), file_path.clone())
            };

            let content = if abs_path.is_file() {
                fs::read(abs_path.as_std_path())?
            } else {
                Vec::new()
            };

            snapshots.push(UndoFileSnapshot {
                path: rel_path,
                content,
            });
        }

        let generation = UndoGeneration {
            id: next_id,
            timestamp,
            op: op.into(),
            files: snapshots,
        };

        let gen_file_name = format!("{next_id:06}.json");
        let gen_path = undo_dir.join(&gen_file_name);
        let json_bytes = serde_json::to_vec_pretty(&generation)
            .map_err(|e| SilError::Parse(format!("failed to serialize undo generation: {e}")))?;

        write_atomic(&gen_path, &json_bytes)?;

        // Enforce maximum generation cap (prune oldest)
        existing.push((next_id, gen_path));
        if existing.len() > Self::MAX_GENERATIONS {
            let to_remove = existing.len() - Self::MAX_GENERATIONS;
            for (_, path) in existing.drain(..to_remove) {
                let _ = fs::remove_file(path.as_std_path());
            }
        }

        Ok(next_id)
    }

    /// Revert the latest generation in `<project_root>/.sil/undo/`, restoring file contents.
    ///
    /// Restores file bytes atomically and removes the popped generation record.
    /// Returns `Ok(Some(generation))` if a generation was restored, or `Ok(None)` if no generations exist.
    pub fn undo(project_root: &Utf8Path) -> Result<Option<UndoGeneration>, SilError> {
        let undo_dir = project_root.join(crate::paths::rel::UNDO);
        if !undo_dir.is_dir() {
            return Ok(None);
        }

        let mut existing = Self::list_generations(&undo_dir)?;
        let Some((_id, latest_path)) = existing.pop() else {
            return Ok(None);
        };

        let data = fs::read(latest_path.as_std_path())?;
        let generation: UndoGeneration = serde_json::from_slice(&data)
            .map_err(|e| SilError::Parse(format!("failed to deserialize undo generation: {e}")))?;

        for snap in &generation.files {
            let target_path = if snap.path.is_absolute() {
                snap.path.clone()
            } else {
                project_root.join(&snap.path)
            };

            if snap.content.is_empty() && !target_path.exists() {
                // Was nonexistent when snapshotted, nothing to restore
            } else {
                write_atomic(&target_path, &snap.content)?;
            }
        }

        let _ = fs::remove_file(latest_path.as_std_path());

        Ok(Some(generation))
    }

    /// List all existing undo generations in `undo_dir` sorted by numeric ID ascending.
    pub fn list_generations(undo_dir: &Utf8Path) -> Result<Vec<(usize, Utf8PathBuf)>, SilError> {
        if !undo_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(undo_dir.as_std_path())? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            // Ignore hidden files and temporary atomic write files
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }

            let stem = name.trim_end_matches(".json");
            if let Ok(id) = stem.parse::<usize>()
                && let Ok(path) = Utf8PathBuf::from_path_buf(entry.path())
            {
                entries.push((id, path));
            }
        }

        entries.sort_by_key(|(id, _)| *id);
        Ok(entries)
    }

    /// Clear all undo generations in `<project_root>/.sil/undo/`.
    pub fn clear(project_root: &Utf8Path) -> Result<(), SilError> {
        let undo_dir = project_root.join(crate::paths::rel::UNDO);
        if undo_dir.is_dir() {
            let _ = fs::remove_dir_all(undo_dir.as_std_path());
        }
        Ok(())
    }
}

/// Create a snapshot of the specified files in the project undo journal.
pub fn snapshot(
    project_root: &Utf8Path,
    op: impl Into<String>,
    files: &[Utf8PathBuf],
) -> Result<usize, SilError> {
    UndoJournal::snapshot(project_root, op, files)
}

/// Revert the latest generation in the project undo journal.
pub fn undo(project_root: &Utf8Path) -> Result<Option<UndoGeneration>, SilError> {
    UndoJournal::undo(project_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_and_undo_restores_exact_file_bytes() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

        let bib_path = root.join("references.bib");
        let initial_bib = "@article{vaswani2017,\n  title={Attention is All You Need}\n}\n";
        fs::write(bib_path.as_std_path(), initial_bib).unwrap();

        // 1. Take snapshot before deleting/modifying
        let id = snapshot(
            &root,
            "Delete bib entry",
            &[Utf8PathBuf::from("references.bib")],
        )
        .unwrap();
        assert_eq!(id, 1);

        // 2. Perform mutation (overwrite / delete content)
        fs::write(bib_path.as_std_path(), "% empty\n").unwrap();
        assert_eq!(
            fs::read_to_string(bib_path.as_std_path()).unwrap(),
            "% empty\n"
        );

        // 3. Undo mutation
        let undone = undo(&root).unwrap().expect("should return generation");
        assert_eq!(undone.id, 1);
        assert_eq!(undone.op, "Delete bib entry");
        assert_eq!(undone.files.len(), 1);
        assert_eq!(undone.files[0].path, "references.bib");

        // 4. Verify exact bytes restored
        let restored = fs::read_to_string(bib_path.as_std_path()).unwrap();
        assert_eq!(restored, initial_bib);

        // 5. Subsequent undo returns None
        assert!(undo(&root).unwrap().is_none());
    }

    #[test]
    fn test_multiple_files_snapshot_and_undo() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

        let bib_path = root.join("references.bib");
        let tex_path = root.join("paper_draft.tex");

        fs::write(bib_path.as_std_path(), "bib content 1").unwrap();
        fs::write(tex_path.as_std_path(), "tex content 1").unwrap();

        let id = snapshot(
            &root,
            "Multi-file edit",
            &[
                Utf8PathBuf::from("references.bib"),
                Utf8PathBuf::from("paper_draft.tex"),
            ],
        )
        .unwrap();
        assert_eq!(id, 1);

        // Mutate both
        fs::write(bib_path.as_std_path(), "mutated bib").unwrap();
        fs::write(tex_path.as_std_path(), "mutated tex").unwrap();

        // Undo
        let undone = undo(&root).unwrap().unwrap();
        assert_eq!(undone.op, "Multi-file edit");
        assert_eq!(
            fs::read_to_string(bib_path.as_std_path()).unwrap(),
            "bib content 1"
        );
        assert_eq!(
            fs::read_to_string(tex_path.as_std_path()).unwrap(),
            "tex content 1"
        );
    }

    #[test]
    fn test_cap_10_generations_prunes_oldest() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let draft_path = root.join("paper_draft.tex");

        for i in 1..=12 {
            fs::write(draft_path.as_std_path(), format!("version {i}")).unwrap();
            let gen_id = snapshot(
                &root,
                format!("Edit {i}"),
                &[Utf8PathBuf::from("paper_draft.tex")],
            )
            .unwrap();
            assert_eq!(gen_id, i);
        }

        let undo_dir = root.join(crate::paths::rel::UNDO);
        let generations = UndoJournal::list_generations(&undo_dir).unwrap();
        assert_eq!(generations.len(), 10);

        // Generations 1 and 2 should have been pruned; generations 3 through 12 remain
        let ids: Vec<usize> = generations.into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, vec![3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);

        // Sequential undos pop in reverse order (12 down to 3)
        for expected_id in (3..=12).rev() {
            let generation = undo(&root).unwrap().expect("expected generation");
            assert_eq!(generation.id, expected_id);
            assert_eq!(generation.op, format!("Edit {expected_id}"));
            assert_eq!(
                fs::read_to_string(draft_path.as_std_path()).unwrap(),
                format!("version {expected_id}")
            );
        }

        // Empty after all undone
        assert!(undo(&root).unwrap().is_none());
    }

    #[test]
    fn test_undo_empty_returns_none() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

        assert!(undo(&root).unwrap().is_none());
    }
}
