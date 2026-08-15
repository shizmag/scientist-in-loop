//! First-party and external skill-pack integration boundaries.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sil_package::{PackageError, PackageSource, sha256_file};
use std::fs;
use thiserror::Error;

use crate::{SkillPackManifest, SkillRegistryError};

/// Checked-in Visualize Article pack manifest.
pub fn visualize_article_manifest() -> Result<SkillPackManifest, SkillRegistryError> {
    let manifest: SkillPackManifest =
        serde_yaml::from_str(include_str!("../packs/visualize-article/skill-pack.yaml"))?;
    manifest.validate()?;
    Ok(manifest)
}

/// Filesystem location of the optional first-party pack source.
pub fn visualize_article_source() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packs/visualize-article")
}

/// Attribution and installation descriptor for ARS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSkillAdapter {
    /// Stable adapter identifier.
    pub id: String,
    /// Pinned adapter metadata revision.
    pub version: String,
    /// Upstream source and exact layout revision.
    pub upstream: PackageSource,
    /// Upstream license identifier and evidence.
    pub license: String,
    /// URL for the upstream license text.
    pub license_evidence: String,
    /// Required attribution text.
    pub attribution: String,
    /// Relative files/layout expected from the user-supplied upstream checkout.
    pub expected_layout: Vec<String>,
    /// Human-readable capability limitation.
    pub capability_report: String,
}

/// Adapter failure, including unavailable user-supplied upstream content.
#[derive(Debug, Error)]
pub enum ExternalSkillError {
    /// Explicit acknowledgement was not supplied.
    #[error("ARS installation requires explicit CC-BY-NC acknowledgement")]
    AcknowledgementRequired,
    /// Upstream checkout/cache is missing or does not match its declared layout.
    #[error("ARS upstream source is unavailable or has an unexpected layout: {0}")]
    Unavailable(String),
    /// Filesystem failure while inspecting a user-supplied checkout.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Package digest failure while auditing first-party files.
    #[error(transparent)]
    Package(#[from] PackageError),
}

/// Return the ARS descriptor without downloading or bundling ARS.
pub fn ars_adapter() -> ExternalSkillAdapter {
    ExternalSkillAdapter {
        id: "external/academic-research-skills".into(),
        version: "1.0.0".into(),
        upstream: PackageSource {
            url: "https://github.com/Imbad0202/academic-research-skills".into(),
            revision: "v3.20.0 (a06529ed68c5709c5f9012ad6c4055a7a0f1ab1e)".into(),
            sha256: "fe9f9851b8c9fff61ebd12e6bf2a1d3536bc25294882ce95b7d280bc91d49c70".into(),
        },
        license: "CC-BY-NC-4.0".into(),
        license_evidence: "https://creativecommons.org/licenses/by-nc/4.0/".into(),
        attribution: "Academic Research Skills by Cheng-I Wu; CC-BY-NC 4.0. Use only under the upstream license.".into(),
        expected_layout: vec!["skills/".into(), "agents/".into()],
        capability_report: "Adapter only: hosts report full, partial, or unsupported subagents, hooks, commands, scripts, and resources; sil does not claim ARS orchestration equivalence.".into(),
    }
}

/// Validate acknowledgement and the layout of a locally supplied ARS checkout.
pub fn validate_ars_source(
    source: &Utf8Path,
    acknowledged: bool,
) -> Result<ExternalSkillAdapter, ExternalSkillError> {
    if !acknowledged {
        return Err(ExternalSkillError::AcknowledgementRequired);
    }
    let descriptor = ars_adapter();
    if !source.is_dir() {
        return Err(ExternalSkillError::Unavailable(source.to_string()));
    }
    for layout in &descriptor.expected_layout {
        if !source.join(layout).is_dir() {
            return Err(ExternalSkillError::Unavailable(format!(
                "missing {}",
                source.join(layout)
            )));
        }
    }
    Ok(descriptor)
}

/// Audit that a directory contains only the declared first-party pack files.
pub fn audit_visualize_article_source() -> Result<(), ExternalSkillError> {
    let root = visualize_article_source();
    for file in ["skill-pack.yaml", "SKILL.md", "NOTICE"] {
        if !root.join(file).is_file() {
            return Err(ExternalSkillError::Unavailable(root.join(file).to_string()));
        }
    }
    let manifest =
        visualize_article_manifest().map_err(|e| ExternalSkillError::Unavailable(e.to_string()))?;
    for file in manifest.files {
        let path = root.join(&file.path);
        if !path.is_file() || sha256_file(path.as_std_path())? != file.sha256 {
            return Err(ExternalSkillError::Unavailable(format!(
                "digest mismatch: {}",
                file.path
            )));
        }
    }
    Ok(())
}

/// Read a local ARS cache marker without treating it as bundled content.
pub fn ars_cache_available(cache: &Utf8Path) -> bool {
    fs::metadata(cache)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityStatus, HostCapabilities, SkillRegistry};

    #[test]
    fn visualize_pack_is_pinned_mit_and_auditable() {
        let manifest = visualize_article_manifest().unwrap();
        assert_eq!(manifest.metadata.id, "scientist-in-loop/visualize-article");
        assert_eq!(manifest.metadata.license, "MIT");
        assert_eq!(manifest.source.revision, "visualize-article-v1.0.0");
        assert!(
            manifest
                .metadata
                .external_data_flow
                .unwrap()
                .consent_required
        );
        audit_visualize_article_source().unwrap();
    }

    #[test]
    fn ars_requires_acknowledgement_and_never_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        assert!(matches!(
            validate_ars_source(&root, false),
            Err(ExternalSkillError::AcknowledgementRequired)
        ));
        assert!(matches!(
            validate_ars_source(&root, true),
            Err(ExternalSkillError::Unavailable(_))
        ));
        assert_eq!(ars_adapter().license, "CC-BY-NC-4.0");
    }

    #[test]
    fn ars_layout_is_user_supplied_and_capability_report_degrades() {
        let dir = tempfile::tempdir().unwrap();
        let source = Utf8PathBuf::from_path_buf(dir.path().join("ars")).unwrap();
        fs::create_dir_all(source.join("skills")).unwrap();
        fs::create_dir_all(source.join("agents")).unwrap();
        let adapter = validate_ars_source(&source, true).unwrap();
        assert!(adapter.capability_report.contains("partial"));

        let project = Utf8PathBuf::from_path_buf(dir.path().join("project")).unwrap();
        let pack_source = visualize_article_source();
        let registry = SkillRegistry::new(project);
        registry.install(&pack_source, true).unwrap();
        let report = registry
            .capability_report(
                "scientist-in-loop/visualize-article",
                &HostCapabilities {
                    network: true,
                    read: ["manuscript".into(), "figures".into()].into(),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(report.status, CapabilityStatus::Partial);
        assert_eq!(report.entrypoints[0].status, CapabilityStatus::Partial);
        assert_eq!(report.entrypoints[0].missing, vec!["resources"]);
        assert_eq!(
            registry
                .check(
                    "scientist-in-loop/visualize-article",
                    &HostCapabilities {
                        network: true,
                        resources: true,
                        read: ["manuscript".into(), "figures".into()].into(),
                        ..Default::default()
                    }
                )
                .unwrap(),
            CapabilityStatus::Full
        );
        assert_eq!(
            registry
                .check(
                    "scientist-in-loop/visualize-article",
                    &HostCapabilities {
                        resources: true,
                        read: ["manuscript".into(), "figures".into()].into(),
                        ..Default::default()
                    }
                )
                .unwrap(),
            CapabilityStatus::Unsupported
        );
    }

    #[test]
    fn first_party_pack_has_no_ars_payload() {
        let root = visualize_article_source();
        let manifest = fs::read_to_string(root.join("skill-pack.yaml")).unwrap();
        let skill = fs::read_to_string(root.join("SKILL.md")).unwrap();
        assert!(!manifest.contains("academic-research-skills"));
        assert!(!skill.contains("academic-research-skills"));
        assert!(!skill.contains("CC-BY-NC"));
    }
}
