//! Versioned skill-pack registry and managed/local projection handling.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sil_package::{
    Compatibility, LicenseMetadata, LockedPackage, ManifestFile, PackageError, PackageKind,
    PackageLock, PackageManifest, PackageSource, confined_path, normalize_relative_path,
    sha256_file,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A skill pack manifest as stored in `skill-pack.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPackManifest {
    /// Manifest schema identifier.
    pub api_version: String,
    /// Must be `SkillPack`.
    pub kind: PackageKind,
    /// Pack identity and license.
    pub metadata: SkillMetadata,
    /// Source provenance.
    pub source: PackageSource,
    /// Host compatibility.
    pub compatibility: Compatibility,
    /// Named entrypoints exposed to hosts.
    pub entrypoints: Vec<SkillEntrypoint>,
    /// Host capabilities requested by this pack.
    pub capabilities: SkillCapabilities,
    /// Complete declared content inventory.
    pub files: Vec<ManifestFile>,
}

/// Skill pack identity and license declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SkillMetadata {
    /// Stable package identifier.
    pub id: String,
    /// Optional human-readable package or skill name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Semver package version.
    pub version: String,
    /// Optional human-readable title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// SPDX license identifier or explicit name.
    pub license: String,
    /// Optional license evidence URL or local path.
    #[serde(default)]
    pub license_evidence: String,
    /// Triggers used for matching tasks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    /// Required toolchain and host capabilities (e.g. "tectonic", "latexmk", "marker", "python", "git", "network").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,
    /// Expected input artifacts or files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Expected output artifacts or files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    /// Permissions needed by the skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// Verification command or action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    /// Conflicting skill IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// Disclosure and acknowledgement metadata for external data flows.
    #[serde(default)]
    pub external_data_flow: Option<ExternalDataFlow>,
}

/// Data-flow disclosure for a skill that sends project data to an external service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalDataFlow {
    /// Named destination class, such as `external_image_provider`.
    pub destination: String,
    /// Data classes that may leave the host.
    pub data_classes: Vec<String>,
    /// Whether the host must obtain explicit user consent before invocation.
    pub consent_required: bool,
    /// Human-readable disclosure shown by host adapters.
    pub disclosure: String,
}

/// One named skill entrypoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SkillEntrypoint {
    /// Stable entrypoint identifier.
    pub id: String,
    /// Optional human-readable entrypoint name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Entrypoint kind, normally `skill`.
    #[serde(rename = "type")]
    pub entrypoint_type: String,
    /// Markdown or other host-readable entrypoint path.
    pub path: String,
    /// Optional human-readable title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Trigger metadata used by routing clients.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
    /// Required capabilities for this entrypoint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,
    /// Expected input artifacts or files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Expected output artifacts or files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    /// Permissions required.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// Verification command or action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    /// Conflicting skill IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// Declared support files/resources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<String>,
    /// Optional host features needed by this entrypoint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_requirements: Vec<String>,
}

/// Capability requirements for a skill pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SkillCapabilities {
    /// Data classes read by the skill.
    #[serde(default)]
    pub read: Vec<String>,
    /// Data classes written by the skill.
    #[serde(default)]
    pub write: Vec<String>,
    /// External network capability, if any.
    #[serde(default)]
    pub network: Option<String>,
    /// Whether process execution is required.
    #[serde(default)]
    pub process: bool,
}

/// Host capabilities used to assess a pack without executing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HostCapabilities {
    /// Supported read data classes.
    #[serde(default)]
    pub read: BTreeSet<String>,
    /// Supported write data classes.
    #[serde(default)]
    pub write: BTreeSet<String>,
    /// Whether external network access is available.
    #[serde(default)]
    pub network: bool,
    /// Whether process execution is available.
    #[serde(default)]
    pub process: bool,
    /// Whether the host can run delegated subagents.
    #[serde(default)]
    pub subagents: bool,
    /// Whether the host can install or invoke lifecycle hooks.
    #[serde(default)]
    pub hooks: bool,
    /// Whether the host exposes named commands.
    #[serde(default)]
    pub commands: bool,
    /// Whether the host can run declared scripts.
    #[serde(default)]
    pub scripts: bool,
    /// Whether the host can expose declared resources.
    #[serde(default)]
    pub resources: bool,
    /// Host identifier used for compatibility matching.
    #[serde(default)]
    pub host: String,
    /// Toolchain and binary capabilities (e.g. "tectonic", "latexmk", "marker", "python", "git").
    #[serde(default)]
    pub tools: BTreeSet<String>,
}

impl HostCapabilities {
    /// Create full permissive capabilities for testing or standard environment.
    pub fn all_available() -> Self {
        let mut tools = BTreeSet::new();
        tools.insert("tectonic".into());
        tools.insert("latexmk".into());
        tools.insert("marker".into());
        tools.insert("python".into());
        tools.insert("git".into());
        Self {
            read: [
                "manuscript".into(),
                "figures".into(),
                "workspace".into(),
                "agent".into(),
            ]
            .into(),
            write: [
                "manuscript".into(),
                "figures".into(),
                "workspace".into(),
                "agent".into(),
            ]
            .into(),
            network: true,
            process: true,
            subagents: true,
            hooks: true,
            commands: true,
            scripts: true,
            resources: true,
            host: "all".into(),
            tools,
        }
    }

    /// Add a tool to the host capabilities.
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.insert(tool.into());
        self
    }

    /// Check whether this host supports a given capability or requirement name.
    pub fn supports(&self, requirement: &str) -> bool {
        let req = requirement.trim().to_ascii_lowercase();
        match req.as_str() {
            "subagents" => self.subagents,
            "hooks" => self.hooks,
            "commands" => self.commands,
            "scripts" => self.scripts,
            "resources" => self.resources,
            "network" => self.network,
            "process" => self.process,
            "git" => self.tools.contains("git") || self.process,
            "python" => self.tools.contains("python") || self.process,
            "tectonic" => self.tools.contains("tectonic"),
            "latexmk" => self.tools.contains("latexmk"),
            "marker" => self.tools.contains("marker"),
            other => {
                if !self.host.is_empty() && self.host.eq_ignore_ascii_case(other) {
                    true
                } else if self.tools.contains(other) {
                    true
                } else if let Some(r) = other.strip_prefix("read:") {
                    self.read.contains(r)
                } else if let Some(w) = other.strip_prefix("write:") {
                    self.write.contains(w)
                } else {
                    false
                }
            }
        }
    }
}

impl From<&sil_core::agent::CapabilitySummary> for HostCapabilities {
    fn from(c: &sil_core::agent::CapabilitySummary) -> Self {
        let mut tools = BTreeSet::new();
        if c.latex_available {
            tools.insert("tectonic".into());
            tools.insert("latexmk".into());
        }
        if c.parser_available {
            tools.insert("marker".into());
        }
        if c.git_available {
            tools.insert("git".into());
        }
        tools.insert("python".into());
        Self {
            network: c.online_search_available || c.llm_provider_available,
            process: true,
            subagents: true,
            hooks: true,
            commands: true,
            scripts: true,
            resources: true,
            host: "standard".into(),
            tools,
            ..Default::default()
        }
    }
}

/// Result of checking one pack against a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    /// Every declared requirement is available.
    Full,
    /// The pack can be exposed with one or more unavailable optional features.
    Partial,
    /// A required host capability is unavailable.
    Unsupported,
}

/// Capability result for one named entrypoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrypointCapabilityReport {
    /// Entrypoint identifier.
    pub entrypoint: String,
    /// Host features that were unavailable.
    pub missing: Vec<String>,
    /// Result for this entrypoint.
    pub status: CapabilityStatus,
}

/// Complete capability report for a pack and host combination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    /// Pack identifier.
    pub pack: String,
    /// Host identifier.
    pub host: String,
    /// Aggregate result.
    pub status: CapabilityStatus,
    /// Per-entrypoint results.
    pub entrypoints: Vec<EntrypointCapabilityReport>,
}

/// A deterministic list entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledSkill {
    /// Pack identifier.
    pub id: String,
    /// Pack version.
    pub version: String,
    /// Entrypoint identifier.
    pub entrypoint: String,
    /// Project-relative managed projection path.
    pub path: String,
}

/// A changed file in an update preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDiff {
    /// Project-relative path.
    pub path: String,
    /// Existing file digest, if present.
    pub old_sha256: Option<String>,
    /// Proposed file digest, if present.
    pub new_sha256: Option<String>,
}

/// Registry failures.
#[derive(Debug, Error)]
pub enum SkillRegistryError {
    /// Invalid manifest, lock, or lifecycle operation.
    #[error("invalid skill registry: {0}")]
    Invalid(String),
    /// Shared package validation failure.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// Filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// YAML failure.
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

impl SkillPackManifest {
    /// Validate the skill schema and all entrypoint/resource confinement rules.
    pub fn validate(&self) -> Result<(), SkillRegistryError> {
        if self.api_version != "sil.dev/skill/v1" || self.kind != PackageKind::SkillPack {
            return Err(SkillRegistryError::Invalid(
                "not a skill-pack manifest".into(),
            ));
        }
        if self.metadata.id.trim().is_empty() || self.metadata.version.trim().is_empty() {
            return Err(SkillRegistryError::Invalid(
                "metadata id/version is required".into(),
            ));
        }
        if self.metadata.license.trim().is_empty()
            || self.metadata.license_evidence.trim().is_empty()
        {
            return Err(SkillRegistryError::Invalid(
                "license and license_evidence are required".into(),
            ));
        }
        if self.capabilities.network.is_some()
            && self
                .metadata
                .external_data_flow
                .as_ref()
                .is_none_or(|flow| {
                    !flow.consent_required
                        || flow.destination.trim().is_empty()
                        || flow.disclosure.trim().is_empty()
                })
        {
            return Err(SkillRegistryError::Invalid(
                "network capability requires explicit external_data_flow consent metadata".into(),
            ));
        }
        if self.entrypoints.is_empty() || self.files.is_empty() {
            return Err(SkillRegistryError::Invalid(
                "entrypoints and files must not be empty".into(),
            ));
        }
        let package = self.package_manifest();
        package.validate()?;
        let declared: BTreeSet<_> = self
            .files
            .iter()
            .map(|f| normalize_relative_path(&f.path))
            .collect::<Result<_, _>>()?;
        let mut ids = BTreeSet::new();
        for entry in &self.entrypoints {
            normalize_relative_path(&entry.path)?;
            if !ids.insert(&entry.id) || !declared.contains(&normalize_relative_path(&entry.path)?)
            {
                return Err(SkillRegistryError::Invalid(format!(
                    "invalid or duplicate entrypoint: {}",
                    entry.id
                )));
            }
            for resource in &entry.resources {
                if !declared.contains(&normalize_relative_path(resource)?) {
                    return Err(SkillRegistryError::Invalid(format!(
                        "undeclared resource: {resource}"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Convert to the shared C1 package envelope.
    pub fn package_manifest(&self) -> PackageManifest {
        PackageManifest {
            api_version: "sil.dev/skill/v1".into(),
            kind: PackageKind::SkillPack,
            package_id: self.metadata.id.clone(),
            version: self.metadata.version.clone(),
            source: self.source.clone(),
            license: LicenseMetadata {
                id: self.metadata.license.clone(),
                evidence: self.metadata.license_evidence.clone(),
            },
            compatibility: self.compatibility.clone(),
            files: self.files.clone(),
            capabilities: capability_names(&self.capabilities),
        }
    }

    /// Convert manifest entrypoints into declarative `SkillDefinition`s.
    pub fn to_skill_definitions(
        &self,
        projection_base: Option<&Utf8Path>,
    ) -> Vec<crate::skills::SkillDefinition> {
        let mut out = Vec::new();
        for entry in &self.entrypoints {
            let path = if let Some(base) = projection_base {
                base.join(&entry.path).to_string()
            } else {
                entry.path.clone()
            };
            let mut triggers = self.metadata.triggers.clone();
            for t in &entry.triggers {
                if !triggers.contains(t) {
                    triggers.push(t.clone());
                }
            }
            let mut required_caps = self.metadata.required_capabilities.clone();
            for c in &entry.required_capabilities {
                if !required_caps.contains(c) {
                    required_caps.push(c.clone());
                }
            }
            for c in &entry.host_requirements {
                if !required_caps.contains(c) {
                    required_caps.push(c.clone());
                }
            }
            if self.capabilities.network.is_some()
                && !required_caps.contains(&"network".to_string())
            {
                required_caps.push("network".to_string());
            }
            if self.capabilities.process && !required_caps.contains(&"process".to_string()) {
                required_caps.push("process".to_string());
            }

            let mut inputs = self.metadata.inputs.clone();
            for i in &entry.inputs {
                if !inputs.contains(i) {
                    inputs.push(i.clone());
                }
            }
            let mut outputs = self.metadata.outputs.clone();
            for o in &entry.outputs {
                if !outputs.contains(o) {
                    outputs.push(o.clone());
                }
            }
            let mut permissions = self.metadata.permissions.clone();
            for p in &entry.permissions {
                if !permissions.contains(p) {
                    permissions.push(p.clone());
                }
            }
            for p in capability_names(&self.capabilities) {
                if !permissions.contains(&p) {
                    permissions.push(p);
                }
            }
            let mut conflicts = self.metadata.conflicts.clone();
            for c in &entry.conflicts {
                if !conflicts.contains(c) {
                    conflicts.push(c.clone());
                }
            }

            out.push(crate::skills::SkillDefinition {
                id: entry.id.clone(),
                name: entry.name.clone().unwrap_or_else(|| {
                    self.metadata
                        .name
                        .clone()
                        .unwrap_or_else(|| entry.id.clone())
                }),
                version: self.metadata.version.clone(),
                title: entry.title.clone().unwrap_or_else(|| {
                    self.metadata
                        .title
                        .clone()
                        .unwrap_or_else(|| entry.id.clone())
                }),
                description: entry
                    .description
                    .clone()
                    .unwrap_or_else(|| self.metadata.description.clone().unwrap_or_default()),
                path,
                triggers,
                required_capabilities: required_caps,
                inputs,
                outputs,
                permissions,
                verification: entry
                    .verification
                    .clone()
                    .or_else(|| self.metadata.verification.clone()),
                conflicts,
            });
        }
        out
    }
}

fn capability_names(c: &SkillCapabilities) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    out.extend(c.read.iter().map(|v| format!("read:{v}")));
    out.extend(c.write.iter().map(|v| format!("write:{v}")));
    if c.network.is_some() {
        out.insert("network".into());
    }
    if c.process {
        out.insert("process".into());
    }
    out
}

/// Project skill registry. Managed content is immutable by convention and local content is never touched.
#[derive(Debug, Clone)]
pub struct SkillRegistry {
    root: Utf8PathBuf,
}

impl SkillRegistry {
    /// Open a registry rooted at a project directory.
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self { root: root.into() }
    }
    /// Return the project root.
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }
    /// Directory for managed projections.
    pub fn managed_dir(&self) -> Utf8PathBuf {
        self.root.join("agent/skills/managed")
    }
    /// Directory for user-authored local skills.
    pub fn local_dir(&self) -> Utf8PathBuf {
        self.root.join("agent/skills/local")
    }
    /// Lock file path.
    pub fn lock_path(&self) -> Utf8PathBuf {
        self.root.join(".sil/skills.lock")
    }

    /// List all locked entrypoints in deterministic order.
    pub fn list(&self) -> Result<Vec<InstalledSkill>, SkillRegistryError> {
        let mut out = Vec::new();
        for lock in self
            .read_lock()?
            .packages
            .into_iter()
            .filter(|p| p.kind == PackageKind::SkillPack)
        {
            let manifest = self.load_manifest(&lock.package_id, &lock.version)?;
            for e in &manifest.entrypoints {
                out.push(InstalledSkill {
                    path: self.projection_path(&manifest, e).to_string(),
                    id: manifest.metadata.id.clone(),
                    version: manifest.metadata.version.clone(),
                    entrypoint: e.id.clone(),
                });
            }
        }
        out.sort_by(|a, b| (&a.id, &a.entrypoint).cmp(&(&b.id, &b.entrypoint)));
        Ok(out)
    }

    /// Show an installed pack manifest.
    pub fn show(&self, id: &str) -> Result<SkillPackManifest, SkillRegistryError> {
        let lock = self.read_lock()?;
        let item = lock
            .packages
            .iter()
            .find(|p| p.package_id == id && p.kind == PackageKind::SkillPack)
            .ok_or_else(|| {
                SkillRegistryError::Invalid(format!("skill pack not installed: {id}"))
            })?;
        self.load_manifest(id, &item.version)
    }

    /// Check compatibility and capability status for an installed pack.
    pub fn check(
        &self,
        id: &str,
        host: &HostCapabilities,
    ) -> Result<CapabilityStatus, SkillRegistryError> {
        let manifest = self.show(id)?;
        if !compatible_with_current_sil(&manifest.compatibility.sil) {
            return Ok(CapabilityStatus::Unsupported);
        }
        Ok(capability_status(&manifest, host))
    }

    /// Report aggregate and per-entrypoint host capability support.
    pub fn capability_report(
        &self,
        id: &str,
        host: &HostCapabilities,
    ) -> Result<CapabilityReport, SkillRegistryError> {
        let manifest = self.show(id)?;
        let entrypoints = manifest
            .entrypoints
            .iter()
            .map(|entry| {
                let missing = entry
                    .host_requirements
                    .iter()
                    .filter(|requirement| !host_supports(host, requirement))
                    .cloned()
                    .collect::<Vec<_>>();
                EntrypointCapabilityReport {
                    entrypoint: entry.id.clone(),
                    status: if missing.is_empty() {
                        CapabilityStatus::Full
                    } else {
                        CapabilityStatus::Partial
                    },
                    missing,
                }
            })
            .collect::<Vec<_>>();
        Ok(CapabilityReport {
            pack: manifest.metadata.id.clone(),
            host: host.host.clone(),
            status: capability_status(&manifest, host),
            entrypoints,
        })
    }

    /// Install a local skill pack after explicit approval.
    pub fn install(
        &self,
        source: &Utf8Path,
        approve: bool,
    ) -> Result<SkillPackManifest, SkillRegistryError> {
        if !approve {
            return Err(SkillRegistryError::Invalid(
                "installation requires --approve".into(),
            ));
        }
        self.install_inner(source, false)
    }

    /// Refuse dirty managed content, back it up, and install a newer revision.
    pub fn update(
        &self,
        source: &Utf8Path,
        approve: bool,
    ) -> Result<SkillPackManifest, SkillRegistryError> {
        if !approve {
            return Err(SkillRegistryError::Invalid(
                "update requires --approve".into(),
            ));
        }
        let incoming = read_manifest(source)?;
        if let Ok(current) = self.show(&incoming.metadata.id) {
            self.ensure_clean(&current)?;
            self.backup(&current)?;
        }
        self.install_inner(source, true)
    }

    /// Preview file changes between an installed pack and a local source.
    pub fn diff(&self, source: &Utf8Path) -> Result<Vec<SkillDiff>, SkillRegistryError> {
        let incoming = read_manifest(source)?;
        let current = self.show(&incoming.metadata.id).ok();
        diff_manifests(self, source, &incoming, current.as_ref())
    }

    /// Verify lock, manifest, projection digests, and compatibility metadata.
    pub fn verify(&self, id: &str) -> Result<(), SkillRegistryError> {
        let manifest = self.show(id)?;
        manifest.validate()?;
        for file in &manifest.files {
            let path = self.projection_path_for_file(&manifest, &file.path);
            if !path.is_file() || sha256_file(Path::new(path.as_str()))? != file.sha256 {
                return Err(SkillRegistryError::Invalid(format!(
                    "managed file is missing or modified: {}",
                    file.path
                )));
            }
        }
        Ok(())
    }

    /// Remove a pack, refusing to delete dirty managed content.
    pub fn remove(&self, id: &str) -> Result<(), SkillRegistryError> {
        let manifest = self.show(id)?;
        self.ensure_clean(&manifest)?;
        let dir = self.pack_dir(&manifest);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        let mut lock = self.read_lock()?;
        lock.packages.retain(|p| p.package_id != id);
        self.write_lock(&lock)
    }

    /// Restore the most recent backed-up version of a pack.
    pub fn rollback(&self, id: &str) -> Result<SkillPackManifest, SkillRegistryError> {
        let root = self.root.join(".sil/skills.rollback").join(escape(id));
        let mut versions: Vec<_> = if root.is_dir() {
            fs::read_dir(&root)?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        } else {
            Vec::new()
        };
        versions.sort();
        let backup = versions
            .pop()
            .ok_or_else(|| SkillRegistryError::Invalid(format!("no rollback available: {id}")))?;
        let _manifest = read_manifest(
            &Utf8PathBuf::from_path_buf(backup.clone())
                .map_err(|_| SkillRegistryError::Invalid("non-UTF-8 rollback path".into()))?,
        )?;
        self.install_inner(
            &Utf8PathBuf::from_path_buf(backup)
                .map_err(|_| SkillRegistryError::Invalid("non-UTF-8 rollback path".into()))?,
            true,
        )
    }

    /// Migrate legacy built-in files without overwriting user changes.
    pub fn migrate_legacy(&self, files: &[(&str, &str)]) -> Result<(), SkillRegistryError> {
        for (relative, content) in files {
            let old = self.root.join("agent/skills").join(relative);
            let managed = self.managed_dir().join(relative);
            let legacy_changed = old.is_file() && fs::read_to_string(old.as_str())? != *content;
            if legacy_changed {
                let backup = self.root.join(".sil/skills-backup/legacy").join(relative);
                if !backup.exists() {
                    if let Some(parent) = backup.parent() {
                        fs::create_dir_all(parent.as_str())?;
                    }
                    fs::copy(old.as_str(), backup.as_str())?;
                }
                let local = self.local_dir().join(relative);
                if !local.exists() {
                    if let Some(parent) = local.parent() {
                        fs::create_dir_all(parent.as_str())?;
                    }
                    fs::copy(old.as_str(), local.as_str())?;
                }
            }
            if !managed.exists() {
                if let Some(parent) = managed.parent() {
                    fs::create_dir_all(parent.as_str())?;
                }
                fs::write(managed.as_str(), content)?;
            } else if fs::read_to_string(managed.as_str())? != *content {
                return Err(SkillRegistryError::Invalid(format!(
                    "dirty managed skill blocks migration: {relative}"
                )));
            }
            // Keep the legacy projection readable for existing agents; user edits
            // are preserved in `local/` before this compatibility refresh.
            if let Some(parent) = old.parent() {
                fs::create_dir_all(parent.as_str())?;
            }
            fs::write(old.as_str(), content)?;
        }
        let package_id = "scientist-in-loop/builtins";
        let version = "1.0.0";
        let manifest = SkillPackManifest {
            api_version: "sil.dev/skill/v1".into(),
            kind: PackageKind::SkillPack,
            metadata: SkillMetadata {
                id: package_id.into(),
                version: version.into(),
                license: "MIT".into(),
                license_evidence: "https://spdx.org/licenses/MIT.html".into(),
                external_data_flow: None,
                ..Default::default()
            },
            source: PackageSource {
                url: "builtin://scientist-in-loop".into(),
                revision: version.into(),
                sha256: sil_package::sha256_bytes(
                    files
                        .iter()
                        .flat_map(|(p, c)| [*p, *c])
                        .collect::<Vec<_>>()
                        .join("\n")
                        .as_bytes(),
                ),
            },
            compatibility: Compatibility {
                sil: ">=1,<2".into(),
                hosts: Vec::new(),
            },
            entrypoints: files
                .iter()
                .map(|(path, _)| SkillEntrypoint {
                    id: path.trim_end_matches(".md").replace('/', "-"),
                    entrypoint_type: "skill".into(),
                    path: (*path).into(),
                    triggers: Vec::new(),
                    ..Default::default()
                })
                .collect(),
            capabilities: SkillCapabilities::default(),
            files: files
                .iter()
                .map(|(path, content)| ManifestFile {
                    path: (*path).into(),
                    sha256: sil_package::sha256_bytes(content.as_bytes()),
                })
                .collect(),
        };
        manifest.validate()?;
        let pack = self.managed_dir().join(escape(package_id)).join(version);
        fs::create_dir_all(pack.as_str())?;
        for (relative, content) in files {
            let path = pack.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent.as_str())?;
            }
            fs::write(path.as_str(), content)?;
        }
        fs::write(
            pack.join("skill-pack.yaml").as_str(),
            serde_yaml::to_string(&manifest)?,
        )?;
        let mut lock = self.read_lock()?;
        lock.packages.retain(|p| p.package_id != package_id);
        lock.packages.push(LockedPackage {
            package_id: package_id.into(),
            version: version.into(),
            kind: PackageKind::SkillPack,
            revision: version.into(),
            sha256: manifest.source.sha256.clone(),
        });
        self.write_lock(&lock)?;
        Ok(())
    }

    fn install_inner(
        &self,
        source: &Utf8Path,
        _update: bool,
    ) -> Result<SkillPackManifest, SkillRegistryError> {
        let manifest = read_manifest(source)?;
        manifest.validate()?;
        validate_files(source, &manifest)?;
        let dest = self.pack_dir(&manifest);
        if dest.exists() {
            fs::remove_dir_all(&dest)?;
        }
        fs::create_dir_all(dest.as_str())?;
        for file in &manifest.files {
            let from = confined_path(Path::new(source.as_str()), &file.path)?;
            let normalized = normalize_relative_path(&file.path)?;
            let parent = Path::new(dest.as_str())
                .join(&normalized)
                .parent()
                .unwrap()
                .to_path_buf();
            fs::create_dir_all(&parent)?;
            let to = confined_path(Path::new(dest.as_str()), &normalized)?;
            fs::copy(from, to)?;
        }
        fs::write(
            dest.join("skill-pack.yaml").as_str(),
            serde_yaml::to_string(&manifest)?,
        )?;
        let mut lock = self.read_lock()?;
        lock.packages
            .retain(|p| p.package_id != manifest.metadata.id);
        lock.packages.push(LockedPackage {
            package_id: manifest.metadata.id.clone(),
            version: manifest.metadata.version.clone(),
            kind: PackageKind::SkillPack,
            revision: manifest.source.revision.clone(),
            sha256: manifest.source.sha256.clone(),
        });
        self.write_lock(&lock)?;
        Ok(manifest)
    }
    fn read_lock(&self) -> Result<PackageLock, SkillRegistryError> {
        let p = self.lock_path();
        if !p.is_file() {
            return Ok(PackageLock::new());
        }
        Ok(PackageLock::from_bytes(&fs::read(p.as_str())?)?)
    }
    fn write_lock(&self, lock: &PackageLock) -> Result<(), SkillRegistryError> {
        if let Some(parent) = self.lock_path().parent() {
            fs::create_dir_all(parent.as_str())?;
        }
        lock.write_atomic(Path::new(self.lock_path().as_str()))?;
        Ok(())
    }
    fn pack_dir(&self, m: &SkillPackManifest) -> Utf8PathBuf {
        self.managed_dir()
            .join(escape(&m.metadata.id))
            .join(&m.metadata.version)
    }
    fn projection_path(&self, m: &SkillPackManifest, e: &SkillEntrypoint) -> Utf8PathBuf {
        self.managed_dir()
            .join(escape(&m.metadata.id))
            .join(&m.metadata.version)
            .join(&e.path)
    }
    fn projection_path_for_file(&self, m: &SkillPackManifest, file: &str) -> Utf8PathBuf {
        self.pack_dir(m).join(file)
    }
    fn load_manifest(
        &self,
        id: &str,
        version: &str,
    ) -> Result<SkillPackManifest, SkillRegistryError> {
        let p = self.managed_dir().join(escape(id)).join(version);
        read_manifest(&p)
    }
    fn ensure_clean(&self, m: &SkillPackManifest) -> Result<(), SkillRegistryError> {
        for file in &m.files {
            let p = self.projection_path_for_file(m, &file.path);
            if !p.is_file() || sha256_file(Path::new(p.as_str()))? != file.sha256 {
                return Err(SkillRegistryError::Invalid(format!(
                    "dirty managed skill blocks update: {}",
                    file.path
                )));
            }
        }
        Ok(())
    }
    fn backup(&self, m: &SkillPackManifest) -> Result<(), SkillRegistryError> {
        let target = self
            .root
            .join(".sil/skills.rollback")
            .join(escape(&m.metadata.id))
            .join(&m.metadata.version);
        if target.exists() {
            return Ok(());
        }
        copy_dir(&self.pack_dir(m), &target)
    }
}

fn escape(id: &str) -> String {
    id.replace('/', "__")
}
fn read_manifest(root: &Utf8Path) -> Result<SkillPackManifest, SkillRegistryError> {
    Ok(serde_yaml::from_str(&fs::read_to_string(
        root.join("skill-pack.yaml").as_str(),
    )?)?)
}
fn validate_files(root: &Utf8Path, m: &SkillPackManifest) -> Result<(), SkillRegistryError> {
    for file in &m.files {
        let p = confined_path(Path::new(root.as_str()), &file.path)?;
        if !p.is_file() || sha256_file(&p)? != file.sha256 {
            return Err(SkillRegistryError::Invalid(format!(
                "file digest mismatch: {}",
                file.path
            )));
        }
    }
    Ok(())
}
fn copy_dir(from: &Utf8Path, to: &Utf8Path) -> Result<(), SkillRegistryError> {
    fs::create_dir_all(to.as_str())?;
    for item in fs::read_dir(from.as_str())? {
        let item = item?;
        let src = item.path();
        let dst = to.join(item.file_name().to_string_lossy().as_ref());
        if src.is_dir() {
            copy_dir(
                &Utf8PathBuf::from_path_buf(src)
                    .map_err(|_| SkillRegistryError::Invalid("non-UTF-8 path".into()))?,
                &dst,
            )?;
        } else {
            fs::copy(src, dst.as_str())?;
        }
    }
    Ok(())
}
fn capability_status(m: &SkillPackManifest, h: &HostCapabilities) -> CapabilityStatus {
    let read_missing = m
        .capabilities
        .read
        .iter()
        .filter(|v| !h.read.contains(*v))
        .count();
    let write_missing = m
        .capabilities
        .write
        .iter()
        .filter(|v| !h.write.contains(*v))
        .count();
    let hard =
        (m.capabilities.network.is_some() && !h.network) || (m.capabilities.process && !h.process);
    let host_missing =
        !m.compatibility.hosts.is_empty() && !m.compatibility.hosts.iter().any(|v| v == &h.host);
    let entrypoint_missing = m.entrypoints.iter().any(|entry| {
        entry
            .host_requirements
            .iter()
            .chain(entry.required_capabilities.iter())
            .any(|requirement| !h.supports(requirement))
    });
    if hard || host_missing {
        CapabilityStatus::Unsupported
    } else if read_missing + write_missing > 0 || entrypoint_missing {
        CapabilityStatus::Partial
    } else {
        CapabilityStatus::Full
    }
}

fn host_supports(host: &HostCapabilities, requirement: &str) -> bool {
    host.supports(requirement)
}
fn diff_manifests(
    registry: &SkillRegistry,
    source: &Utf8Path,
    incoming: &SkillPackManifest,
    current: Option<&SkillPackManifest>,
) -> Result<Vec<SkillDiff>, SkillRegistryError> {
    incoming.validate()?;
    validate_files(source, incoming)?;
    let old: BTreeMap<_, _> = current
        .map(|m| {
            m.files
                .iter()
                .map(|f| (f.path.clone(), f.sha256.clone()))
                .collect()
        })
        .unwrap_or_default();
    let new: BTreeMap<_, _> = incoming
        .files
        .iter()
        .map(|f| (f.path.clone(), f.sha256.clone()))
        .collect();
    let paths: BTreeSet<_> = old.keys().chain(new.keys()).cloned().collect();
    let mut out = Vec::new();
    for path in paths {
        let old_sha = old.get(&path).cloned();
        let new_sha = new.get(&path).cloned();
        if old_sha != new_sha {
            let _ = (registry, source);
            out.push(SkillDiff {
                path,
                old_sha256: old_sha,
                new_sha256: new_sha,
            });
        }
    }
    Ok(out)
}

fn compatible_with_current_sil(range: &str) -> bool {
    let version = 1u64;
    range.split(',').all(|clause| {
        let clause = clause.trim();
        let (operator, value) = if let Some(value) = clause.strip_prefix(">=") {
            (">=", value)
        } else if let Some(value) = clause.strip_prefix("<=") {
            ("<=", value)
        } else if let Some(value) = clause.strip_prefix('>') {
            (">", value)
        } else if let Some(value) = clause.strip_prefix('<') {
            ("<", value)
        } else {
            ("=", clause)
        };
        let Some(required) = value
            .trim()
            .split('.')
            .next()
            .and_then(|v| v.parse::<u64>().ok())
        else {
            return false;
        };
        match operator {
            ">=" => version >= required,
            "<=" => version <= required,
            ">" => version > required,
            "<" => version < required,
            _ => version == required,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_pack(root: &Utf8Path, version: &str, body: &str) {
        fs::create_dir_all(root.join("resources/nested").as_str()).unwrap();
        fs::write(root.join("main.md").as_str(), body).unwrap();
        fs::write(root.join("resources/nested/data.txt").as_str(), "resource").unwrap();
        let files = ["main.md", "resources/nested/data.txt"];
        let manifest = SkillPackManifest {
            api_version: "sil.dev/skill/v1".into(),
            kind: PackageKind::SkillPack,
            metadata: SkillMetadata {
                id: "example/pack".into(),
                version: version.into(),
                license: "MIT".into(),
                license_evidence: "https://spdx.org/licenses/MIT.html".into(),
                external_data_flow: None,
                ..Default::default()
            },
            source: PackageSource {
                url: "file://fixture".into(),
                revision: version.into(),
                sha256: sil_package::sha256_bytes(version.as_bytes()),
            },
            compatibility: Compatibility {
                sil: ">=1,<2".into(),
                hosts: vec!["test".into()],
            },
            entrypoints: vec![SkillEntrypoint {
                id: "nested-skill".into(),
                entrypoint_type: "skill".into(),
                path: "main.md".into(),
                triggers: vec!["nested".into()],
                ..Default::default()
            }],
            capabilities: SkillCapabilities {
                read: vec!["manuscript".into()],
                ..Default::default()
            },
            files: files
                .iter()
                .map(|file| ManifestFile {
                    path: (*file).into(),
                    sha256: sha256_file(Path::new(root.join(file).as_str())).unwrap(),
                })
                .collect(),
        };
        fs::write(
            root.join("skill-pack.yaml").as_str(),
            serde_yaml::to_string(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn routes_nested_entrypoints_and_resources() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let source = Utf8PathBuf::from_path_buf(dir.path().join("pack")).unwrap();
        make_pack(&source, "1.0.0", "hello");
        let registry = SkillRegistry::new(&root);
        registry.install(&source, true).unwrap();
        assert_eq!(registry.list().unwrap()[0].entrypoint, "nested-skill");
        assert_eq!(
            fs::read_to_string(registry.list().unwrap()[0].path.as_str()).unwrap(),
            "hello"
        );
        assert_eq!(
            registry
                .check(
                    "example/pack",
                    &HostCapabilities {
                        host: "test".into(),
                        read: ["manuscript".into()].into(),
                        ..Default::default()
                    }
                )
                .unwrap(),
            CapabilityStatus::Full
        );
    }

    #[test]
    fn dirty_update_refused_and_local_survives() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let first = Utf8PathBuf::from_path_buf(dir.path().join("first")).unwrap();
        let second = Utf8PathBuf::from_path_buf(dir.path().join("second")).unwrap();
        make_pack(&first, "1.0.0", "one");
        make_pack(&second, "2.0.0", "two");
        let registry = SkillRegistry::new(&root);
        registry.install(&first, true).unwrap();
        fs::create_dir_all(registry.local_dir().as_str()).unwrap();
        fs::write(registry.local_dir().join("mine.md").as_str(), "mine").unwrap();
        let managed = registry.list().unwrap()[0].path.clone();
        fs::write(managed.as_str(), "edited").unwrap();
        assert!(registry.update(&second, true).is_err());
        assert_eq!(
            fs::read_to_string(registry.local_dir().join("mine.md").as_str()).unwrap(),
            "mine"
        );
    }

    #[test]
    fn diff_update_and_rollback_are_explicit() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let first = Utf8PathBuf::from_path_buf(dir.path().join("first")).unwrap();
        let second = Utf8PathBuf::from_path_buf(dir.path().join("second")).unwrap();
        make_pack(&first, "1.0.0", "one");
        make_pack(&second, "2.0.0", "two");
        let registry = SkillRegistry::new(&root);
        registry.install(&first, true).unwrap();
        assert_eq!(registry.diff(&second).unwrap().len(), 1);
        registry.update(&second, true).unwrap();
        assert_eq!(
            registry.show("example/pack").unwrap().metadata.version,
            "2.0.0"
        );
        registry.rollback("example/pack").unwrap();
        assert_eq!(
            registry.show("example/pack").unwrap().metadata.version,
            "1.0.0"
        );
    }

    #[test]
    fn visualize_article_pack_has_notice_and_no_ars_content() {
        let notice = include_str!("../packs/visualize-article/NOTICE");
        let skill = include_str!("../packs/visualize-article/SKILL.md");
        let manifest = include_str!("../packs/visualize-article/skill-pack.yaml");
        assert!(notice.contains("MIT"));
        assert!(skill.contains("external") || manifest.contains("external_data_flow"));
        assert!(!skill.contains("CC-BY-NC"));
        assert!(!manifest.contains("CC-BY-NC"));
    }

    #[test]
    fn to_skill_definitions_and_capability_checks() {
        let manifest_yaml = r#"
api_version: sil.dev/skill/v1
kind: skill-pack
metadata:
  id: custom/pack
  name: custom-pack
  version: 1.5.0
  title: Custom Analysis Pack
  description: Performs deep statistical analysis and plotting.
  license: MIT
  license_evidence: https://spdx.org/licenses/MIT.html
  triggers:
    - analysis
    - statistics
  required_capabilities:
    - python
    - git
  inputs:
    - data/raw.csv
  outputs:
    - results/summary.json
  permissions:
    - read:data
    - write:results
  verification: verify_stats
  conflicts:
    - legacy/pack
entrypoints:
  - id: analyze
    type: skill
    path: analyze.md
    title: Run Statistical Analysis
    triggers:
      - regression
    required_capabilities:
      - R
files:
  - path: analyze.md
    sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
capabilities:
  read:
    - data
  write:
    - results
  network: external_api
  process: true
"#;
        let manifest: SkillPackManifest = serde_yaml::from_str(manifest_yaml).unwrap();
        manifest.validate().unwrap();

        let defs = manifest.to_skill_definitions(None);
        assert_eq!(defs.len(), 1);
        let def = &defs[0];
        assert_eq!(def.id, "analyze");
        assert_eq!(def.name, "custom-pack");
        assert_eq!(def.version, "1.5.0");
        assert_eq!(def.title, "Run Statistical Analysis");
        assert!(def.triggers.contains(&"analysis".to_string()));
        assert!(def.triggers.contains(&"regression".to_string()));
        assert!(def.required_capabilities.contains(&"python".to_string()));
        assert!(def.required_capabilities.contains(&"git".to_string()));
        assert!(def.required_capabilities.contains(&"R".to_string()));
        assert!(def.required_capabilities.contains(&"network".to_string()));
        assert!(def.required_capabilities.contains(&"process".to_string()));
        assert!(def.inputs.contains(&"data/raw.csv".to_string()));
        assert!(def.outputs.contains(&"results/summary.json".to_string()));
        assert!(def.conflicts.contains(&"legacy/pack".to_string()));
        assert_eq!(def.verification.as_deref(), Some("verify_stats"));

        let host = HostCapabilities {
            tools: ["python".into(), "git".into()].into(),
            network: true,
            process: true,
            ..Default::default()
        };

        // Host is missing R
        assert!(!host.supports("R"));
        assert!(host.supports("python"));
        assert!(host.supports("git"));
        assert!(host.supports("network"));
        assert!(host.supports("process"));
    }
}
