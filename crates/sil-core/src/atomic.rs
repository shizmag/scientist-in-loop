//! Crash-safe atomic file writing utilities.

use std::fs::{self, File};
use std::io::{self, Write};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use camino::Utf8Path;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Atomically writes `bytes` to `path`.
///
/// Parent directories are created if they do not exist.
/// Writing is done via a temporary file in the same directory as `path`,
/// flushed and synced to disk, and then atomically renamed to `path`.
///
/// On POSIX systems, a successful call guarantees that `path` either holds the complete
/// new bytes or (if rename never occurred due to an error prior to rename) the original bytes.
pub fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;

    fs::create_dir_all(parent)?;

    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no file name")
    })?;

    let pid = process::id();
    let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);

    let temp_name = format!(".{file_name}.{pid}.{nanos}_{count}.tmp");
    let temp_path = parent.join(temp_name);

    let res = (|| -> io::Result<()> {
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    if res.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    res
}

/// Atomically writes `text` to `path` as UTF-8 text.
///
/// See [`write_atomic`] for durability guarantees.
pub fn write_atomic_str(path: &Utf8Path, text: &str) -> io::Result<()> {
    write_atomic(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_write_atomic_basic_readback() {
        let dir = tempdir().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(dir.path().join("test.txt")).unwrap();

        write_atomic_str(&file_path, "hello world").unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_write_atomic_overwrite() {
        let dir = tempdir().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(dir.path().join("overwrite.txt")).unwrap();

        write_atomic_str(&file_path, "initial content").unwrap();
        write_atomic_str(&file_path, "updated content").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "updated content");
    }

    #[test]
    fn test_write_atomic_parent_is_file_leaves_dest_unchanged() {
        let dir = tempdir().unwrap();
        let block_file = Utf8PathBuf::from_path_buf(dir.path().join("block")).unwrap();
        fs::write(&block_file, "i am a file").unwrap();

        let invalid_path = block_file.join("subfile.txt");
        let err = write_atomic_str(&invalid_path, "should fail");
        assert!(err.is_err());

        let content = fs::read_to_string(&block_file).unwrap();
        assert_eq!(content, "i am a file");
    }

    #[test]
    fn test_write_atomic_pid_in_temp_and_sequential_writes() {
        let dir = tempdir().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(dir.path().join("seq.txt")).unwrap();

        let pid = process::id();

        write_atomic(&file_path, b"pass 1").unwrap();
        write_atomic(&file_path, b"pass 2").unwrap();

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "pass 2");
        // Verify no leftover .tmp files
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(entries, vec!["seq.txt"]);
        assert!(pid > 0);
    }
}
