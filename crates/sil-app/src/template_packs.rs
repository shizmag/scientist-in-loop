//! Validated template-pack installation and isolated manuscript staging.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sil_package::{
    Compatibility, LicenseMetadata, LockedPackage, ManifestFile, PackageError, PackageKind,
    PackageLock, PackageManifest, PackageSource, confined_path, normalize_relative_path,
    sha256_file,
};
use thiserror::Error;

/// A template redistribution permission. Unknown values are retained and fail closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedistributionPermission {
    /// The operation is permitted after verification.
    Allowed,
    /// Only bytes explicitly supplied by the user may be used.
    UserSuppliedOnly,
    /// The operation is prohibited.
    Forbidden,
    /// An unknown permission, which is never accepted for a privileged operation.
    #[serde(other)]
    Unknown,
}

/// Redistribution declarations from `template.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Redistribution {
    /// Whether the pack may be bundled with sil.
    pub bundled_with_sil: RedistributionPermission,
    /// Whether the pack may be retained in the local cache.
    pub local_cache: RedistributionPermission,
    /// Whether the pack may be included in a release archive.
    pub release_archive: RedistributionPermission,
    /// Evidence for the redistribution terms.
    pub evidence: String,
}

/// Template metadata from the package manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Stable template pack identifier.
    pub id: String,
    /// Semver template pack version.
    pub version: String,
    /// Declared license identifier.
    pub license: String,
    /// Official repository or project page.
    pub repository: String,
}

/// Safe manuscript insertion adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateAdapter {
    /// Adapter implementation identifier.
    pub id: String,
    /// Literal anchor that occurs exactly once in the entrypoint.
    pub content_anchor: String,
}

/// Template build declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateBuild {
    /// Supported local engines.
    pub engines: Vec<String>,
    /// Expected output PDF name.
    pub expected_pdf: String,
}

/// Submission constraints exposed to later validation/release stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TemplateConstraints {
    /// Whether anonymous submissions are supported.
    #[serde(default)]
    pub anonymous: Option<bool>,
    /// Optional page limit.
    #[serde(default)]
    pub max_pages: Option<u32>,
}

/// Normative template package manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateManifest {
    /// Versioned manifest API.
    pub api_version: String,
    /// Package kind, normally `TemplatePack`.
    pub kind: PackageKind,
    /// Template metadata.
    pub metadata: TemplateMetadata,
    /// Immutable source provenance.
    pub source: TemplateSource,
    /// License and redistribution declaration.
    pub redistribution: Redistribution,
    /// Complete payload inventory.
    pub files: Vec<ManifestFile>,
    /// Entrypoint relative to the pack root.
    pub entrypoint: String,
    /// Manuscript adapter.
    pub adapter: TemplateAdapter,
    /// Build declaration.
    pub build: TemplateBuild,
    /// Compatibility and submission constraints.
    pub constraints: TemplateConstraints,
}

/// Source section of a template manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSource {
    /// Exact source revision.
    pub revision: String,
    /// Source digest.
    pub sha256: String,
}

impl TemplateManifest {
    /// Convert the template-specific manifest to the common package envelope.
    pub fn package_manifest(&self) -> PackageManifest {
        PackageManifest {
            api_version: self.api_version.clone(),
            kind: self.kind,
            package_id: self.metadata.id.clone(),
            version: self.metadata.version.clone(),
            source: PackageSource {
                url: self.metadata.repository.clone(),
                revision: self.source.revision.clone(),
                sha256: self.source.sha256.clone(),
            },
            license: LicenseMetadata {
                id: self.metadata.license.clone(),
                evidence: self.redistribution.evidence.clone(),
            },
            compatibility: Compatibility {
                sil: ">=1,<2".into(),
                hosts: Vec::new(),
            },
            files: self.files.clone(),
            capabilities: BTreeSet::new(),
        }
    }

    /// Validate schema and all template-specific invariants.
    pub fn validate(&self) -> Result<(), TemplateError> {
        self.package_manifest().validate()?;
        if self.kind != PackageKind::TemplatePack || self.api_version != "sil.dev/template/v1" {
            return Err(TemplateError::Invalid(
                "manifest is not a template pack".into(),
            ));
        }
        normalize_relative_path(&self.entrypoint)?;
        if self.adapter.id != "latex-anchor-v1" || self.adapter.content_anchor.trim().is_empty() {
            return Err(TemplateError::Invalid(
                "unsupported or empty adapter".into(),
            ));
        }
        if self.build.engines.is_empty() || self.build.expected_pdf.trim().is_empty() {
            return Err(TemplateError::Invalid(
                "build declaration is incomplete".into(),
            ));
        }
        if !self.files.iter().any(|f| f.path == self.entrypoint) {
            return Err(TemplateError::Invalid(
                "entrypoint is not declared in files".into(),
            ));
        }
        if self.redistribution.evidence.trim().is_empty()
            || matches!(
                self.redistribution.local_cache,
                RedistributionPermission::Forbidden | RedistributionPermission::Unknown
            )
        {
            return Err(TemplateError::Invalid(
                "local cache permission is not granted".into(),
            ));
        }
        Ok(())
    }
}

/// Errors from template-pack operations.
#[derive(Debug, Error)]
pub enum TemplateError {
    /// Invalid manifest or unsafe operation.
    #[error("invalid template pack: {0}")]
    Invalid(String),
    /// Common package validation failure.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// Filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// YAML failure.
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

/// Summary of an installed pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledTemplate {
    /// Stable pack identifier.
    pub id: String,
    /// Installed version.
    pub version: String,
    /// Cache path.
    pub cache_path: Utf8PathBuf,
}

/// Install and stage packs for one project.
#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    root: Utf8PathBuf,
}

impl TemplateRegistry {
    /// Open the template registry rooted at a project.
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn base(&self) -> Utf8PathBuf {
        self.root.join(".sil/templates")
    }
    fn cache(&self, m: &TemplateManifest) -> Utf8PathBuf {
        self.base().join("cache").join(format!(
            "{}-{}",
            m.metadata.id.replace('/', "__"),
            m.metadata.version
        ))
    }
    fn lock_path(&self) -> Utf8PathBuf {
        self.root.join(".sil/template.lock")
    }

    /// List exact lock entries currently installed.
    pub fn list(&self) -> Result<Vec<InstalledTemplate>, TemplateError> {
        let lock = self.read_lock()?;
        Ok(lock
            .packages
            .into_iter()
            .map(|p| InstalledTemplate {
                cache_path: self.base().join("cache").join(format!(
                    "{}-{}",
                    p.package_id.replace('/', "__"),
                    p.version
                )),
                id: p.package_id,
                version: p.version,
            })
            .collect())
    }

    /// Read and validate an installed manifest.
    pub fn show(&self, id: &str) -> Result<TemplateManifest, TemplateError> {
        self.load_installed(id).map(|(_, m)| m)
    }

    /// Install a local directory pack and atomically update `.sil/template.lock`.
    pub fn install(
        &self,
        source: &Utf8Path,
        approve: bool,
    ) -> Result<InstalledTemplate, TemplateError> {
        if !approve {
            return Err(TemplateError::Invalid(
                "installation requires --approve".into(),
            ));
        }
        let source_path = source
            .as_str()
            .strip_prefix("file://")
            .map(Utf8PathBuf::from)
            .unwrap_or_else(|| source.to_path_buf());
        let (source_root, manifest) = read_pack(&source_path)?;
        manifest.validate()?;
        let cache = self.cache(&manifest);
        if cache.exists() {
            verify_pack(&cache, &manifest)?;
        } else {
            copy_tree(&source_root, &cache)?;
        }
        verify_pack(&cache, &manifest)?;
        let mut lock = self.read_lock()?;
        lock.packages
            .retain(|p| p.package_id != manifest.metadata.id);
        lock.packages.push(LockedPackage {
            package_id: manifest.metadata.id.clone(),
            version: manifest.metadata.version.clone(),
            kind: PackageKind::TemplatePack,
            revision: manifest.source.revision.clone(),
            sha256: manifest.source.sha256.clone(),
        });
        fs::create_dir_all(self.base().join("cache"))?;
        lock.write_atomic(Path::new(self.lock_path().as_str()))?;
        Ok(InstalledTemplate {
            id: manifest.metadata.id,
            version: manifest.metadata.version,
            cache_path: cache,
        })
    }

    /// Update is install with an explicit local source.
    pub fn update(
        &self,
        source: &Utf8Path,
        approve: bool,
    ) -> Result<InstalledTemplate, TemplateError> {
        self.install(source, approve)
    }

    /// Verify the locked pack and its immutable cache.
    pub fn verify(&self, id: &str) -> Result<(), TemplateError> {
        let (root, m) = self.load_installed(id)?;
        verify_pack(&root, &m)
    }

    /// Remove a pack from the lock and cache.
    pub fn remove(&self, id: &str) -> Result<(), TemplateError> {
        let mut lock = self.read_lock()?;
        let old = lock.packages.len();
        let removed = lock.packages.iter().find(|p| p.package_id == id).cloned();
        lock.packages.retain(|p| p.package_id != id);
        if old == lock.packages.len() {
            return Err(TemplateError::Invalid(format!(
                "template not installed: {id}"
            )));
        }
        lock.write_atomic(Path::new(self.lock_path().as_str()))?;
        if let Some(p) = removed {
            let cache = self.base().join("cache").join(format!(
                "{}-{}",
                p.package_id.replace('/', "__"),
                p.version
            ));
            if cache.is_dir() {
                fs::remove_dir_all(cache.as_str())?;
            }
        }
        Ok(())
    }

    /// Stage the template and manuscript into an isolated, exact tree.
    pub fn stage(
        &self,
        id: &str,
        manuscript: &Utf8Path,
        output: Option<&Utf8Path>,
    ) -> Result<Utf8PathBuf, TemplateError> {
        let (root, m) = self.load_installed(id)?;
        verify_pack(&root, &m)?;
        let text = fs::read_to_string(manuscript)?;
        let entry = confined_path(Path::new(root.as_str()), &m.entrypoint)?;
        let template = fs::read_to_string(entry)?;
        let count = template.match_indices(&m.adapter.content_anchor).count();
        if count != 1 {
            return Err(TemplateError::Invalid(format!(
                "content anchor occurs {count} times"
            )));
        }
        let rendered = template.replace(&m.adapter.content_anchor, &text);
        let destination = output.map(Utf8PathBuf::from).unwrap_or_else(|| {
            self.base()
                .join("staging")
                .join(m.metadata.id.replace('/', "__"))
        });
        if destination.exists() {
            fs::remove_dir_all(destination.as_str())?;
        }
        fs::create_dir_all(destination.as_str())?;
        for file in &m.files {
            let src = confined_path(Path::new(root.as_str()), &file.path)?;
            let dst = destination.join(&file.path);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent.as_str())?;
            }
            if file.path == m.entrypoint {
                fs::write(dst.as_str(), rendered.as_bytes())?;
            } else {
                fs::copy(src, dst.as_str())?;
            }
        }
        Ok(destination)
    }

    fn read_lock(&self) -> Result<PackageLock, TemplateError> {
        let p = self.lock_path();
        if !p.is_file() {
            return Ok(PackageLock::new());
        }
        Ok(PackageLock::from_bytes(&fs::read(p.as_str())?)?)
    }
    fn load_installed(&self, id: &str) -> Result<(Utf8PathBuf, TemplateManifest), TemplateError> {
        let lock = self.read_lock()?;
        let p = lock
            .packages
            .iter()
            .find(|p| p.package_id == id)
            .ok_or_else(|| TemplateError::Invalid(format!("template not installed: {id}")))?;
        let root = self.base().join("cache").join(format!(
            "{}-{}",
            p.package_id.replace('/', "__"),
            p.version
        ));
        let (_, m) = read_pack(&root)?;
        if m.source.sha256 != p.sha256 || m.source.revision != p.revision {
            return Err(TemplateError::Invalid("cache does not match lock".into()));
        }
        Ok((root, m))
    }
}

fn read_pack(root: &Utf8Path) -> Result<(Utf8PathBuf, TemplateManifest), TemplateError> {
    let root = Utf8PathBuf::from_path_buf(root.canonicalize().map_err(TemplateError::Io)?)
        .map_err(|_| TemplateError::Invalid("template path is not valid UTF-8".into()))?;
    let bytes = fs::read(root.join("template.yaml"))?;
    let manifest: TemplateManifest = serde_yaml::from_slice(&bytes)?;
    Ok((root, manifest))
}

fn verify_pack(root: &Utf8Path, manifest: &TemplateManifest) -> Result<(), TemplateError> {
    manifest.validate()?;
    let mut actual = BTreeSet::new();
    collect_payload(
        Path::new(root.as_str()),
        Path::new(root.as_str()),
        &mut actual,
    )?;
    let declared: BTreeSet<_> = manifest
        .files
        .iter()
        .map(|f| normalize_relative_path(&f.path))
        .collect::<Result<_, _>>()?;
    if actual != declared {
        return Err(TemplateError::Invalid(
            "cache files do not match manifest".into(),
        ));
    }
    for f in &manifest.files {
        let p = confined_path(Path::new(root.as_str()), &f.path)?;
        if sha256_file(&p)? != f.sha256 {
            return Err(TemplateError::Invalid(format!("hash mismatch: {}", f.path)));
        }
    }
    Ok(())
}

fn collect_payload(
    root: &Path,
    current: &Path,
    out: &mut BTreeSet<String>,
) -> Result<(), TemplateError> {
    for e in fs::read_dir(current)? {
        let p = e?.path();
        if p.file_name().is_some_and(|n| n == "template.yaml") {
            continue;
        }
        if p.is_symlink() {
            return Err(TemplateError::Invalid("symlink in template pack".into()));
        }
        if p.is_dir() {
            collect_payload(root, &p, out)?;
        } else if p.is_file() {
            out.insert(
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
    Ok(())
}

fn copy_tree(source: &Utf8Path, destination: &Utf8Path) -> Result<(), TemplateError> {
    fs::create_dir_all(destination.as_str())?;
    for e in fs::read_dir(source.as_str())? {
        let e = e?;
        let src = e.path();
        let dst = destination.join(e.file_name().to_string_lossy().as_ref());
        if src.is_symlink() {
            return Err(TemplateError::Invalid("symlink in template pack".into()));
        }
        if src.is_dir() {
            copy_tree(Utf8Path::from_path(&src).unwrap(), &dst)?;
        } else {
            fs::copy(src, dst.as_str())?;
        }
    }
    Ok(())
}
