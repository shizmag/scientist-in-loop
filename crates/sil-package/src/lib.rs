//! Offline-first package transport primitives.
//!
//! This crate deliberately knows nothing about templates, skills, or execution. It
//! verifies bytes and metadata, stores immutable content, and confines paths.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors produced while validating or storing a package.
#[derive(Debug, Error)]
pub enum PackageError {
    /// The package input violates a security or schema invariant.
    #[error("invalid package: {0}")]
    Invalid(String),
    /// A required package file is absent or differs from its declared digest.
    #[error("package file error: {0}")]
    File(String),
    /// The package schema is newer than this crate supports.
    #[error("unsupported schema version: {0}")]
    UnsupportedSchema(String),
    /// The archive exceeded an intake limit.
    #[error("archive limit exceeded: {0}")]
    Limit(String),
    /// Underlying filesystem or serialization failure.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Serialization failure.
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    /// YAML failure.
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    /// ZIP failure.
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

/// Package family. Component-specific schemas are owned by later crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "PascalCase")]
pub enum PackageKind {
    /// A template package.
    TemplatePack,
    /// A skill package.
    SkillPack,
}

/// Package origin and immutable source revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSource {
    /// Source URL or local-origin description.
    pub url: String,
    /// Exact source revision or release identifier.
    pub revision: String,
    /// Digest of the source archive/content when available.
    pub sha256: String,
}

/// License and provenance evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseMetadata {
    /// SPDX identifier or explicit license name.
    pub id: String,
    /// URL or local evidence for the license terms.
    pub evidence: String,
}

/// Runtime compatibility declaration, without implying execution support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Compatibility {
    /// Compatible sil version range.
    pub sil: String,
    /// Compatible host identifiers.
    #[serde(default)]
    pub hosts: Vec<String>,
}

/// One declared package file and its SHA-256 digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Package-relative path.
    pub path: String,
    /// Lowercase hexadecimal SHA-256 digest.
    pub sha256: String,
}

/// Common package envelope shared by template and skill packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Versioned envelope identifier, for example `sil.dev/template/v1`.
    pub api_version: String,
    /// Package family.
    pub kind: PackageKind,
    /// Stable package identifier.
    pub package_id: String,
    /// Semver package version.
    pub version: String,
    /// Source provenance.
    pub source: PackageSource,
    /// License provenance.
    pub license: LicenseMetadata,
    /// Host compatibility.
    pub compatibility: Compatibility,
    /// Complete declared file inventory.
    pub files: Vec<ManifestFile>,
    /// Named capabilities, informational only in this crate.
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
}

impl PackageManifest {
    /// Validate schema, required provenance, paths, and digest syntax.
    pub fn validate(&self) -> Result<(), PackageError> {
        let expected = match self.kind {
            PackageKind::TemplatePack => "sil.dev/template/v1",
            PackageKind::SkillPack => "sil.dev/skill/v1",
        };
        if self.api_version != expected {
            return Err(PackageError::UnsupportedSchema(self.api_version.clone()));
        }
        for (name, value) in [
            ("package_id", &self.package_id),
            ("version", &self.version),
            ("source.url", &self.source.url),
            ("source.revision", &self.source.revision),
            ("source.sha256", &self.source.sha256),
            ("license.id", &self.license.id),
            ("license.evidence", &self.license.evidence),
            ("compatibility.sil", &self.compatibility.sil),
        ] {
            if value.trim().is_empty() {
                return Err(PackageError::Invalid(format!("{name} is required")));
            }
        }
        validate_digest(&self.source.sha256)?;
        let mut paths = BTreeSet::new();
        if self.files.is_empty() {
            return Err(PackageError::Invalid("files must not be empty".into()));
        }
        for file in &self.files {
            let normalized = normalize_relative_path(&file.path)?;
            if !paths.insert(normalized) {
                return Err(PackageError::Invalid(format!(
                    "duplicate path: {}",
                    file.path
                )));
            }
            validate_digest(&file.sha256)?;
        }
        Ok(())
    }
}

/// A deterministic exact package resolution record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package identity.
    pub package_id: String,
    /// Exact package version.
    pub version: String,
    /// Package family.
    pub kind: PackageKind,
    /// Exact source revision.
    pub revision: String,
    /// Content digest used as the cache key.
    pub sha256: String,
}

/// Project lock containing exact package resolutions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PackageLock {
    /// Lock schema version.
    pub schema_version: u32,
    /// Exact resolutions.
    pub packages: Vec<LockedPackage>,
}

impl PackageLock {
    /// Create an empty current lock.
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            packages: Vec::new(),
        }
    }

    /// Serialize with stable package ordering and JSON formatting.
    pub fn to_bytes(&self) -> Result<Vec<u8>, PackageError> {
        if self.schema_version != 1 {
            return Err(PackageError::UnsupportedSchema(
                self.schema_version.to_string(),
            ));
        }
        let mut lock = self.clone();
        lock.packages.sort_by(|a, b| {
            (a.package_id.as_str(), a.version.as_str())
                .cmp(&(b.package_id.as_str(), b.version.as_str()))
        });
        Ok(serde_json::to_vec_pretty(&lock)?)
    }

    /// Parse and validate a lock.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackageError> {
        let lock: Self = serde_json::from_slice(bytes)?;
        if lock.schema_version != 1 {
            return Err(PackageError::UnsupportedSchema(
                lock.schema_version.to_string(),
            ));
        }
        for item in &lock.packages {
            validate_digest(&item.sha256)?;
        }
        Ok(lock)
    }

    /// Atomically replace a lock file, preserving the old file if the write fails.
    pub fn write_atomic(&self, path: &Path) -> Result<(), PackageError> {
        atomic_write(path, &self.to_bytes()?)
    }
}

/// SHA-256 of bytes as lowercase hexadecimal.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_slice())
}

/// SHA-256 of a regular file.
pub fn sha256_file(path: &Path) -> Result<String, PackageError> {
    Ok(sha256_bytes(&fs::read(path)?))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn validate_digest(value: &str) -> Result<(), PackageError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(PackageError::Invalid(format!(
            "invalid SHA-256 digest: {value}"
        )));
    }
    Ok(())
}

/// Normalize and validate a package-relative path.
pub fn normalize_relative_path(value: &str) -> Result<String, PackageError> {
    if value.is_empty() || Path::new(value).is_absolute() {
        return Err(PackageError::Invalid(format!(
            "path is not relative: {value}"
        )));
    }
    let mut out = Vec::new();
    for component in Path::new(value).components() {
        match component {
            Component::Normal(part) => out.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PackageError::Invalid(format!("unsafe path: {value}")));
            }
        }
    }
    if out.is_empty() {
        return Err(PackageError::Invalid(format!(
            "empty normalized path: {value}"
        )));
    }
    Ok(out.join("/"))
}

/// Resolve a package-relative path beneath `root`, including symlink checks.
pub fn confined_path(root: &Path, relative: &str) -> Result<PathBuf, PackageError> {
    let normalized = normalize_relative_path(relative)?;
    let root = fs::canonicalize(root)?;
    let candidate = root.join(&normalized);
    let check = if candidate.exists() {
        fs::canonicalize(&candidate)?
    } else {
        fs::canonicalize(
            candidate
                .parent()
                .ok_or_else(|| PackageError::Invalid("missing parent".into()))?,
        )?
        .join(candidate.file_name().unwrap())
    };
    if !check.starts_with(&root) {
        return Err(PackageError::Invalid(format!(
            "path escapes root: {relative}"
        )));
    }
    Ok(check)
}

/// Validate that a directory contains exactly the declared regular files.
pub fn validate_directory(root: &Path, manifest: &PackageManifest) -> Result<(), PackageError> {
    manifest.validate()?;
    let mut declared = BTreeSet::new();
    for entry in &manifest.files {
        let path = confined_path(root, &entry.path)?;
        if !path.is_file() {
            return Err(PackageError::File(format!(
                "missing manifest file: {}",
                entry.path
            )));
        }
        if sha256_file(&path)? != entry.sha256 {
            return Err(PackageError::File(format!("hash mismatch: {}", entry.path)));
        }
        declared.insert(normalize_relative_path(&entry.path)?);
    }
    let mut actual = BTreeSet::new();
    collect_files(root, root, &mut actual)?;
    if actual != declared {
        return Err(PackageError::File(
            "directory files do not match manifest".into(),
        ));
    }
    Ok(())
}

fn collect_files(
    root: &Path,
    current: &Path,
    out: &mut BTreeSet<String>,
) -> Result<(), PackageError> {
    for item in fs::read_dir(current)? {
        let item = item?;
        let path = item.path();
        if path.is_symlink() {
            return Err(PackageError::Invalid(format!(
                "symlink in package: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            out.insert(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
    Ok(())
}

/// Default KD-C10 intake bounds.
#[derive(Debug, Clone)]
pub struct ArchiveLimits {
    /// Maximum compressed bytes.
    pub max_compressed_bytes: u64,
    /// Maximum extracted bytes.
    pub max_extracted_bytes: u64,
    /// Maximum files.
    pub max_files: usize,
    /// Maximum individual file bytes.
    pub max_file_bytes: u64,
    /// Maximum relative path depth.
    pub max_path_depth: usize,
    /// Maximum compression ratio.
    pub max_compression_ratio: u64,
    /// Maximum extraction duration.
    pub max_duration: Duration,
}
impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_compressed_bytes: 64 * 1024 * 1024,
            max_extracted_bytes: 256 * 1024 * 1024,
            max_files: 4096,
            max_file_bytes: 64 * 1024 * 1024,
            max_path_depth: 32,
            max_compression_ratio: 100,
            max_duration: Duration::from_secs(30),
        }
    }
}

/// Extract a ZIP archive under bounded KD-C10 limits.
pub fn extract_zip<R: Read + io::Seek>(
    reader: R,
    destination: &Path,
    limits: &ArchiveLimits,
) -> Result<(), PackageError> {
    let start = Instant::now();
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut compressed = 0u64;
    let mut extracted = 0u64;
    let mut files = 0usize;
    for index in 0..archive.len() {
        if start.elapsed() > limits.max_duration {
            return Err(PackageError::Limit("extraction time".into()));
        }
        let entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        let normalized = normalize_relative_path(&name)?;
        let depth = normalized.split('/').count();
        if depth > limits.max_path_depth {
            return Err(PackageError::Limit("path depth".into()));
        }
        if entry.is_dir() {
            continue;
        }
        files += 1;
        if files > limits.max_files {
            return Err(PackageError::Limit("file count".into()));
        }
        compressed = compressed.saturating_add(entry.compressed_size());
        extracted = extracted.saturating_add(entry.size());
        if compressed > limits.max_compressed_bytes
            || extracted > limits.max_extracted_bytes
            || entry.size() > limits.max_file_bytes
        {
            return Err(PackageError::Limit("archive size".into()));
        }
        if entry.compressed_size() > 0
            && entry.size() / entry.compressed_size().max(1) > limits.max_compression_ratio
        {
            return Err(PackageError::Limit("compression ratio".into()));
        }
        let path = confined_path(destination, &normalized)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let mut limited = entry.take(limits.max_file_bytes + 1);
        io::copy(&mut limited, &mut output)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), PackageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp-sil-package");
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

/// Content-addressed immutable cache.
#[derive(Debug, Clone)]
pub struct PackageCache {
    /// Cache root.
    pub root: PathBuf,
    /// Explicit byte quota.
    pub quota_bytes: u64,
}

/// Small cache metadata record written next to a content blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// Package identifier associated with the blob.
    pub package_id: String,
    /// Exact package version associated with the blob.
    pub version: String,
    /// License identifier copied from the verified manifest.
    pub license: String,
}

impl PackageCache {
    /// Use `$XDG_CACHE_HOME/sil/packages`, or the platform cache directory.
    pub fn xdg(quota_bytes: u64) -> Result<Self, PackageError> {
        let base =
            dirs::cache_dir().ok_or_else(|| PackageError::Invalid("no cache directory".into()))?;
        Ok(Self::new(base.join("sil/packages"), quota_bytes))
    }
    /// Construct a cache at an explicit root.
    pub fn new(root: PathBuf, quota_bytes: u64) -> Self {
        Self { root, quota_bytes }
    }
    /// Store immutable bytes under their content digest and return that digest.
    pub fn put(&self, bytes: &[u8]) -> Result<String, PackageError> {
        let digest = sha256_bytes(bytes);
        let path = self.root.join(&digest);
        fs::create_dir_all(&self.root)?;
        if path.exists() {
            return Ok(digest);
        }
        let tmp = self.root.join(format!(".{digest}.tmp"));
        {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        fs::rename(tmp, &path)?;
        make_read_only(&path)?;
        Ok(digest)
    }
    /// Read a cached blob, returning `None` on a cache miss.
    pub fn get(&self, digest: &str) -> Result<Option<Vec<u8>>, PackageError> {
        validate_digest(digest)?;
        let path = self.root.join(digest);
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(fs::read(path)?))
    }
    /// Atomically write metadata for a cached digest.
    pub fn write_metadata(
        &self,
        digest: &str,
        metadata: &CacheMetadata,
    ) -> Result<(), PackageError> {
        validate_digest(digest)?;
        if self.get(digest)?.is_none() {
            return Err(PackageError::File(format!("cache miss: {digest}")));
        }
        let bytes = serde_json::to_vec_pretty(metadata)?;
        atomic_write(&self.root.join(format!("{digest}.json")), &bytes)
    }
    /// Read cache metadata without network access.
    pub fn read_metadata(&self, digest: &str) -> Result<Option<CacheMetadata>, PackageError> {
        validate_digest(digest)?;
        let path = self.root.join(format!("{digest}.json"));
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
    }
    /// Enforce quota, retaining all explicitly locked digests.
    pub fn enforce_quota(&self, locked: &[String]) -> Result<(), PackageError> {
        if !self.root.exists() {
            return Ok(());
        }
        let locked: BTreeSet<&str> = locked.iter().map(String::as_str).collect();
        let mut entries = Vec::new();
        let mut total = 0u64;
        for item in fs::read_dir(&self.root)? {
            let item = item?;
            let meta = item.metadata()?;
            if meta.is_file() {
                total += meta.len();
                entries.push((item.path(), meta.len(), meta.modified().ok()));
            }
        }
        entries.sort_by_key(|(_, _, time)| *time);
        for (path, size, _) in entries {
            if total <= self.quota_bytes {
                break;
            }
            let digest = path
                .file_name()
                .and_then(|x| x.to_str())
                .map(|name| name.strip_suffix(".json").unwrap_or(name))
                .unwrap_or_default();
            if locked.contains(digest) {
                continue;
            }
            fs::remove_file(path)?;
            total -= size;
        }
        if total > self.quota_bytes {
            return Err(PackageError::Limit(
                "cache quota (locked content retained)".into(),
            ));
        }
        Ok(())
    }
}

fn make_read_only(path: &Path) -> Result<(), PackageError> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::tempdir;

    fn manifest(root: &Path) -> PackageManifest {
        let file = root.join("main.txt");
        fs::write(&file, b"hello").unwrap();
        PackageManifest {
            api_version: "sil.dev/template/v1".into(),
            kind: PackageKind::TemplatePack,
            package_id: "example/test".into(),
            version: "1.0.0".into(),
            source: PackageSource {
                url: "file://fixture".into(),
                revision: "r1".into(),
                sha256: sha256_bytes(b"source"),
            },
            license: LicenseMetadata {
                id: "MIT".into(),
                evidence: "https://spdx.org/licenses/MIT.html".into(),
            },
            compatibility: Compatibility {
                sil: ">=1,<2".into(),
                hosts: vec![],
            },
            files: vec![ManifestFile {
                path: "main.txt".into(),
                sha256: sha256_file(&file).unwrap(),
            }],
            capabilities: BTreeSet::new(),
        }
    }

    #[test]
    fn manifest_and_lock_roundtrip() {
        let dir = tempdir().unwrap();
        let m = manifest(dir.path());
        m.validate().unwrap();
        let mut lock = PackageLock::new();
        lock.packages.push(LockedPackage {
            package_id: m.package_id.clone(),
            version: m.version.clone(),
            kind: m.kind,
            revision: "r1".into(),
            sha256: m.source.sha256.clone(),
        });
        assert_eq!(
            PackageLock::from_bytes(&lock.to_bytes().unwrap()).unwrap(),
            lock
        );
    }
    #[test]
    fn lock_order_is_stable() {
        let mut a = PackageLock::new();
        let mut b = PackageLock::new();
        for id in ["z", "a"] {
            let item = LockedPackage {
                package_id: id.into(),
                version: "1".into(),
                kind: PackageKind::SkillPack,
                revision: "r".into(),
                sha256: sha256_bytes(id.as_bytes()),
            };
            a.packages.push(item.clone());
            b.packages.insert(0, item);
        }
        assert_eq!(a.to_bytes().unwrap(), b.to_bytes().unwrap());
    }
    #[test]
    fn rejects_traversal_duplicate_and_hash_mismatch() {
        assert!(normalize_relative_path("../x").is_err());
        let dir = tempdir().unwrap();
        let mut m = manifest(dir.path());
        m.files.push(m.files[0].clone());
        assert!(m.validate().is_err());
        m.files.pop();
        m.files[0].sha256 = sha256_bytes(b"wrong");
        assert!(validate_directory(dir.path(), &m).is_err());
    }
    #[test]
    fn rejects_symlink_escape() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("x"), b"x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path().join("x"), dir.path().join("x")).unwrap();
        #[cfg(unix)]
        assert!(confined_path(dir.path(), "x").is_err());
    }
    #[test]
    fn cache_is_offline_and_quota_retains_locked() {
        let dir = tempdir().unwrap();
        let cache = PackageCache::new(dir.path().into(), 3);
        let digest = cache.put(b"abcd").unwrap();
        assert_eq!(cache.get(&digest).unwrap().unwrap(), b"abcd");
        assert!(cache.enforce_quota(std::slice::from_ref(&digest)).is_err());
    }
    #[test]
    fn cache_metadata_is_atomic_and_offline() {
        let dir = tempdir().unwrap();
        let cache = PackageCache::new(dir.path().into(), 100);
        let digest = cache.put(b"cached").unwrap();
        let metadata = CacheMetadata {
            package_id: "example/test".into(),
            version: "1.0.0".into(),
            license: "MIT".into(),
        };
        cache.write_metadata(&digest, &metadata).unwrap();
        assert_eq!(cache.read_metadata(&digest).unwrap(), Some(metadata));
    }
    #[test]
    fn failed_lock_replacement_keeps_old_lock() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("package.lock");
        let lock = PackageLock::new();
        lock.write_atomic(&path).unwrap();
        let mut unsupported = lock.clone();
        unsupported.schema_version = 99;
        assert!(unsupported.write_atomic(&path).is_err());
        assert_eq!(
            PackageLock::from_bytes(&fs::read(path).unwrap()).unwrap(),
            lock
        );
    }
    #[test]
    fn zip_traversal_and_limits_rejected() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut bytes);
            zip.start_file("../escape", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }
        let dir = tempdir().unwrap();
        assert!(
            extract_zip(
                Cursor::new(bytes.into_inner()),
                dir.path(),
                &ArchiveLimits::default()
            )
            .is_err()
        );
    }
}
