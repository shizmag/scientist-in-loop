//! Deterministic, comment-aware static analysis of a LaTeX manuscript.
//!
//! The parser intentionally supports static brace arguments only. Dynamic
//! macro-generated paths, citations, labels, and assets are reported as
//! unsupported observations rather than expanded.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sil_core::{
    CheckFinding, CheckProfile, CheckStaticReport, FindingClass, input_fingerprint,
    sort_and_deduplicate,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::LatexError;

/// Additional roots which may be used by graphics and input references.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphOptions {
    /// Project root. All returned project-relative paths are relative to this directory.
    pub project_root: Utf8PathBuf,
    /// Main manuscript file, relative to `project_root` or absolute.
    pub main: Utf8PathBuf,
    /// Bibliography files to inspect. `\\bibliography` references are added automatically.
    pub bibliography: Vec<Utf8PathBuf>,
    /// Explicitly permitted roots for external dependencies.
    pub allowed_roots: Vec<Utf8PathBuf>,
    /// Graphics suffixes tried when the reference has no suffix.
    pub graphic_extensions: Vec<String>,
}

impl GraphOptions {
    /// Construct options using the conventional project layout.
    pub fn new(project_root: impl Into<Utf8PathBuf>, main: impl Into<Utf8PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            main: main.into(),
            bibliography: Vec::new(),
            allowed_roots: Vec::new(),
            graphic_extensions: ["pdf", "png", "jpg", "jpeg", "eps"]
                .map(str::to_string)
                .into(),
        }
    }
}

/// The type of a resolved manuscript dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Configured main manuscript.
    Main,
    /// `\\input` dependency.
    Input,
    /// `\\include` dependency.
    Include,
    /// `\\includegraphics` dependency.
    Graphic,
    /// `\\usepackage` dependency.
    Style,
    /// `\\documentclass` dependency.
    Class,
    /// BibTeX or biblatex bibliography dependency.
    Bibliography,
}

/// One canonical dependency in stable traversal order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyNode {
    /// Canonical path, project-relative unless it is outside the project.
    pub path: String,
    /// Dependency role.
    pub kind: DependencyKind,
    /// Path of the file containing the reference.
    pub referenced_from: Option<String>,
    /// Whether the dependency exists.
    pub exists: bool,
    /// Whether it lies outside the project root.
    pub external: bool,
}

/// A citation occurrence and its nearby source context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationContext {
    /// Citation key.
    pub key: String,
    /// Citation macro (without a star).
    pub macro_name: String,
    /// Source file.
    pub path: String,
    /// One-based source line.
    pub line: usize,
    /// Text on the source line with the citation removed.
    pub context: String,
}

/// A label definition or reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelReference {
    /// Label key.
    pub key: String,
    /// Source file.
    pub path: String,
    /// One-based source line.
    pub line: usize,
}

/// A graphic occurrence, including the resolved candidate path when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReference {
    /// Original argument to `\\includegraphics`.
    pub requested: String,
    /// Resolved canonical path, if a permitted candidate exists.
    pub resolved: Option<String>,
    /// Source file.
    pub path: String,
    /// One-based source line.
    pub line: usize,
}

/// Complete deterministic manuscript snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySnapshot {
    /// Schema version.
    pub schema_version: u32,
    /// Files reached from the configured main file.
    pub dependencies: Vec<DependencyNode>,
    /// Citation occurrences.
    pub citations: Vec<CitationContext>,
    /// Label definitions.
    pub labels: Vec<LabelReference>,
    /// Label references.
    pub references: Vec<LabelReference>,
    /// Graphic occurrences.
    pub assets: Vec<AssetReference>,
    /// Canonical roots outside the project used by references.
    pub external_roots: Vec<String>,
    /// A1-compatible static report containing current-state findings.
    pub report: CheckStaticReport,
}

/// Build a dependency graph and current-state manuscript findings.
pub fn build_dependency_graph(options: &GraphOptions) -> Result<DependencySnapshot, LatexError> {
    let root = canonical_or_absolute(Path::new(options.project_root.as_str()));
    let main = resolve_path(&root, Path::new(options.main.as_str()));
    if !main.is_file() {
        return Err(LatexError::MainNotFound(main.display().to_string()));
    }
    let permitted = permitted_roots(&root, options);
    let mut state = State::new(options, root.clone(), permitted);
    visit(&mut state, &main, DependencyKind::Main, None)?;
    for bib in &options.bibliography {
        let mut path = resolve_path(&root, Path::new(bib.as_str()));
        if path.extension().is_none() {
            path.set_extension("bib");
        }
        visit(&mut state, &path, DependencyKind::Bibliography, Some(&main))?;
        if path.is_file() && state.bib_parsed.insert(canonical_or_absolute(&path)) {
            parse_bib(&mut state, &path)?;
        }
    }
    let mut findings = state.findings;
    for (key, locations) in &state.bib_keys {
        if locations.len() > 1 {
            findings.push(finding(
                "latex.bib.key_duplicate",
                FindingClass::InvariantError,
                locations[1].clone(),
                None,
                format!("BibTeX key '{key}' is defined more than once"),
                json!({"key": key, "count": locations.len()}),
            ));
        }
    }
    for (key, locations) in &state.labels {
        if locations.len() > 1 {
            findings.push(finding(
                "latex.label.duplicate",
                FindingClass::InvariantError,
                locations[1].0.clone(),
                Some(locations[1].1),
                format!("Label '{key}' is defined more than once"),
                json!({"label": key, "count": locations.len()}),
            ));
        }
    }
    let defined: BTreeSet<_> = state.labels.keys().cloned().collect();
    for r in &state.references {
        if !defined.contains(&r.key) {
            findings.push(finding(
                "latex.reference.undefined",
                FindingClass::ActionableWarning,
                r.path.clone(),
                Some(r.line),
                format!("Reference '{0}' targets an undefined label", r.key),
                json!({"label": r.key}),
            ));
        }
    }
    let bib: BTreeSet<_> = state.bib_keys.keys().cloned().collect();
    for c in &state.citations {
        if !bib.contains(&c.key) {
            findings.push(finding(
                "latex.citation.undefined",
                FindingClass::ActionableWarning,
                c.path.clone(),
                Some(c.line),
                format!(
                    "Citation key '{0}' is not defined in the bibliography",
                    c.key
                ),
                json!({"key": c.key}),
            ));
        }
    }
    findings.sort_by_key(|f| (f.path.clone(), f.line, f.code.clone(), f.message.clone()));
    sort_and_deduplicate(&mut findings);
    let input = serde_json::to_vec(&(
        &state.nodes,
        &state.citations,
        &state.labels,
        &state.references,
        &state.assets,
    ))
    .map_err(|e| LatexError::Io {
        path: "<graph>".into(),
        source: std::io::Error::other(e),
    })?;
    let mut report =
        CheckStaticReport::new(CheckProfile::Draft, input_fingerprint(&input), findings);
    report.dependencies = state.nodes.iter().map(|n| n.path.clone()).collect();
    report
        .metrics
        .insert("citations".into(), json!(state.citations.len()));
    report
        .metrics
        .insert("labels".into(), json!(state.labels.len()));
    report
        .metrics
        .insert("assets".into(), json!(state.assets.len()));
    Ok(DependencySnapshot {
        schema_version: 1,
        dependencies: state.nodes,
        citations: state.citations,
        labels: flatten(state.labels),
        references: state.references,
        assets: state.assets,
        external_roots: state.external_roots.into_iter().collect(),
        report,
    })
}

struct State<'a> {
    options: &'a GraphOptions,
    root: PathBuf,
    permitted: Vec<PathBuf>,
    nodes: Vec<DependencyNode>,
    citations: Vec<CitationContext>,
    labels: BTreeMap<String, Vec<(String, usize)>>,
    references: Vec<LabelReference>,
    assets: Vec<AssetReference>,
    bib_keys: BTreeMap<String, Vec<String>>,
    bib_parsed: HashSet<PathBuf>,
    external_roots: BTreeSet<String>,
    active: HashSet<PathBuf>,
    seen: HashSet<PathBuf>,
    findings: Vec<CheckFinding>,
}

impl<'a> State<'a> {
    fn new(options: &'a GraphOptions, root: PathBuf, permitted: Vec<PathBuf>) -> Self {
        Self {
            options,
            root,
            permitted,
            nodes: Vec::new(),
            citations: Vec::new(),
            labels: BTreeMap::new(),
            references: Vec::new(),
            assets: Vec::new(),
            bib_keys: BTreeMap::new(),
            bib_parsed: HashSet::new(),
            external_roots: BTreeSet::new(),
            active: HashSet::new(),
            seen: HashSet::new(),
            findings: Vec::new(),
        }
    }
}

fn visit(
    s: &mut State<'_>,
    path: &Path,
    kind: DependencyKind,
    from: Option<&Path>,
) -> Result<(), LatexError> {
    let canonical = canonical_or_absolute(path);
    let display = display_path(&s.root, &canonical);
    if !s.permitted.iter().any(|r| canonical.starts_with(r)) {
        s.findings.push(finding(
            "latex.dependency.path_escape",
            FindingClass::InvariantError,
            display.clone(),
            None,
            "Dependency resolves outside permitted roots".into(),
            json!({"path": display}),
        ));
        return Ok(());
    }
    if kind != DependencyKind::Main && s.active.contains(&canonical) {
        s.findings.push(finding(
            "latex.dependency.cycle",
            FindingClass::InvariantError,
            display.clone(),
            None,
            "Cyclic TeX dependency detected".into(),
            json!({"path": display}),
        ));
        return Ok(());
    }
    if s.seen.insert(canonical.clone()) {
        let external = !canonical.starts_with(&s.root);
        if external && let Some(parent) = canonical.parent() {
            s.external_roots.insert(parent.display().to_string());
        }
        s.nodes.push(DependencyNode {
            path: display.clone(),
            kind,
            referenced_from: from.map(|p| display_path(&s.root, p)),
            exists: canonical.is_file(),
            external,
        });
    }
    if !canonical.is_file() {
        if kind != DependencyKind::Main {
            s.findings.push(finding(
                "latex.dependency.missing",
                FindingClass::InvariantError,
                display,
                None,
                "Referenced TeX dependency is missing".into(),
                json!({"path": path.display().to_string()}),
            ));
        }
        return Ok(());
    }
    if !s.active.insert(canonical.clone()) {
        return Ok(());
    }
    let text = fs::read_to_string(&canonical).map_err(|e| LatexError::Io {
        path: canonical.display().to_string(),
        source: e,
    })?;
    let clean = strip_comments(&text);
    parse_content(s, &canonical, &clean)?;
    s.active.remove(&canonical);
    Ok(())
}

fn parse_content(s: &mut State<'_>, file: &Path, text: &str) -> Result<(), LatexError> {
    let path = display_path(&s.root, file);
    let graphic_dirs = graphic_paths(text, file);
    for (line, command, arg) in commands(text, &["input", "include"]) {
        let target = resolve_tex_path(file.parent().unwrap_or(file), Path::new(&arg));
        visit(
            s,
            &target,
            if command == "input" {
                DependencyKind::Input
            } else {
                DependencyKind::Include
            },
            Some(file),
        )?;
        if !target.is_file() {
            let _ = line;
        }
    }
    for (_, _, arg) in commands(text, &["bibliography", "addbibresource"]) {
        for bib in arg.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            let mut p = resolve_path(file.parent().unwrap_or(file), Path::new(bib));
            if p.extension().is_none() {
                p.set_extension("bib");
            }
            visit(s, &p, DependencyKind::Bibliography, Some(file))?;
            if p.is_file() && s.bib_parsed.insert(canonical_or_absolute(&p)) {
                parse_bib(s, &p)?;
            }
        }
    }
    for (line, macro_name, arg) in commands(
        text,
        &[
            "cite",
            "nocite",
            "autocite",
            "parencite",
            "textcite",
            "citep",
            "citet",
            "citealp",
            "citeauthor",
            "citeyear",
            "footcite",
            "smartcite",
            "supercite",
        ],
    ) {
        for key in arg.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            s.citations.push(CitationContext {
                key: key.into(),
                macro_name: macro_name.clone(),
                path: path.clone(),
                line,
                context: text
                    .lines()
                    .nth(line.saturating_sub(1))
                    .unwrap_or("")
                    .trim()
                    .into(),
            });
        }
    }
    for (line, _, arg) in commands(text, &["label"]) {
        if !arg.trim().is_empty() {
            s.labels
                .entry(arg.trim().into())
                .or_default()
                .push((path.clone(), line));
        }
    }
    for (line, _, arg) in commands(
        text,
        &[
            "ref", "pageref", "autoref", "cref", "Cref", "vref", "eqref", "nameref",
        ],
    ) {
        if !arg.trim().is_empty() {
            s.references.push(LabelReference {
                key: arg.trim().into(),
                path: path.clone(),
                line,
            });
        }
    }
    for (line, _, arg) in commands(text, &["includegraphics"]) {
        let resolved = resolve_graphic(s, file, &graphic_dirs, &arg);
        let requested_path = canonical_or_absolute(&resolve_path(
            file.parent().unwrap_or(file),
            Path::new(&arg),
        ));
        if !s
            .permitted
            .iter()
            .any(|root| requested_path.starts_with(root))
        {
            s.findings.push(finding(
                "latex.dependency.path_escape",
                FindingClass::InvariantError,
                path.clone(),
                Some(line),
                "Graphic asset resolves outside permitted roots".into(),
                json!({"requested": arg}),
            ));
        }
        if resolved.is_none() {
            s.findings.push(finding(
                "latex.asset.missing",
                FindingClass::ActionableWarning,
                path.clone(),
                Some(line),
                format!("Graphic asset '{arg}' could not be resolved"),
                json!({"requested": arg}),
            ));
        }
        s.assets.push(AssetReference {
            requested: arg,
            resolved: resolved.as_ref().map(|p| display_path(&s.root, p)),
            path: path.clone(),
            line,
        });
        let dependency_path = resolved.unwrap_or(requested_path);
        let dependency_display = display_path(&s.root, &dependency_path);
        if !dependency_path.starts_with(&s.root)
            && let Some(parent) = dependency_path.parent()
        {
            s.external_roots.insert(parent.display().to_string());
        }
        if !s
            .nodes
            .iter()
            .any(|node| node.path == dependency_display && node.kind == DependencyKind::Graphic)
        {
            let external = !dependency_path.starts_with(&s.root);
            s.nodes.push(DependencyNode {
                path: dependency_display,
                kind: DependencyKind::Graphic,
                referenced_from: Some(path.clone()),
                exists: dependency_path.is_file(),
                external,
            });
        }
    }
    for (_, _, arg) in commands(text, &["usepackage"]) {
        for package in arg.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            let path = resolve_path(file.parent().unwrap_or(file), Path::new(package));
            let path = if path.extension().is_none() {
                path.with_extension("sty")
            } else {
                path
            };
            visit(s, &path, DependencyKind::Style, Some(file))?;
        }
    }
    for (_, _, arg) in commands(text, &["documentclass"]) {
        let path = resolve_path(file.parent().unwrap_or(file), Path::new(arg.trim()));
        let path = if path.extension().is_none() {
            path.with_extension("cls")
        } else {
            path
        };
        visit(s, &path, DependencyKind::Class, Some(file))?;
    }
    Ok(())
}

fn parse_bib(s: &mut State<'_>, path: &Path) -> Result<(), LatexError> {
    let text = fs::read_to_string(path).map_err(|e| LatexError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    for (line, raw) in text.lines().enumerate() {
        let t = raw.trim();
        if t.starts_with('@')
            && let Some(open) = t.find(['{', '('])
            && let Some(comma) = t[open + 1..].find(',')
        {
            let key = t[open + 1..open + 1 + comma].trim();
            if !key.is_empty() {
                s.bib_keys.entry(key.into()).or_default().push(format!(
                    "{}:{}",
                    display_path(&s.root, path),
                    line + 1
                ));
            }
        }
    }
    Ok(())
}

fn commands(text: &str, names: &[&str]) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(found) = text[pos..].find('\\') {
        let start = pos + found + 1;
        let rest = &text[start..];
        let name_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        if name_len == 0 {
            pos = start;
            continue;
        }
        let name = &rest[..name_len];
        pos = start + name_len;
        if !names.contains(&name) {
            continue;
        }
        let mut cursor = pos;
        while text[cursor..].starts_with([' ', '\t', '\r', '\n', '*']) {
            cursor += 1;
        }
        while text[cursor..].starts_with('[') {
            if let Some(end) = balanced_end(text, cursor, '[', ']') {
                cursor = end + 1;
            } else {
                break;
            }
            while text[cursor..].starts_with([' ', '\t', '\r', '\n']) {
                cursor += 1;
            }
        }
        if text[cursor..].starts_with('{')
            && let Some(end) = balanced_end(text, cursor, '{', '}')
        {
            let line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
            out.push((line, name.into(), text[cursor + 1..end].into()));
            pos = end + 1;
        }
    }
    out
}

fn balanced_end(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for (offset, c) in text[start..].char_indices() {
        if c == open {
            depth += 1;
        }
        if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(start + offset);
            }
        }
    }
    None
}

fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| {
            let mut escaped = false;
            let mut end = line.len();
            for (i, c) in line.char_indices() {
                if c == '%' && !escaped {
                    end = i;
                    break;
                }
                escaped = c == '\\' && !escaped;
                if c != '\\' {
                    escaped = false;
                }
            }
            &line[..end]
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn graphic_paths(text: &str, file: &Path) -> Vec<PathBuf> {
    commands(text, &["graphicspath"])
        .into_iter()
        .flat_map(|(_, _, arg)| {
            arg.split('{')
                .filter_map(|v| v.strip_suffix('}').map(str::to_string))
                .collect::<Vec<_>>()
        })
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map(|p| resolve_path(file.parent().unwrap_or(file), &p))
        .collect()
}
fn resolve_graphic(s: &State<'_>, file: &Path, dirs: &[PathBuf], arg: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for dir in dirs {
        candidates.push(dir.join(arg));
    }
    candidates.push(file.parent().unwrap_or(file).join(arg));
    candidates.push(s.root.join(arg));
    for ext in &s.options.graphic_extensions {
        if Path::new(arg).extension().is_none() {
            for p in candidates.clone() {
                candidates.push(p.with_extension(ext));
            }
        }
    }
    candidates
        .into_iter()
        .map(|p| canonical_or_absolute(&p))
        .find(|p| p.is_file() && s.permitted.iter().any(|r| p.starts_with(r)))
}
fn permitted_roots(root: &Path, o: &GraphOptions) -> Vec<PathBuf> {
    let mut roots = vec![root.to_path_buf()];
    roots.extend(
        o.allowed_roots
            .iter()
            .map(|p| canonical_or_absolute(Path::new(p.as_str()))),
    );
    roots
}
fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn resolve_tex_path(base: &Path, path: &Path) -> PathBuf {
    let resolved = resolve_path(base, path);
    if resolved.extension().is_none() {
        let with_extension = resolved.with_extension("tex");
        if with_extension.is_file() {
            return with_extension;
        }
    }
    resolved
}
fn canonical_or_absolute(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| lexical_normalize(path))
}
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            _ => out.push(c.as_os_str()),
        }
    }
    out
}
fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string())
}
fn flatten(map: BTreeMap<String, Vec<(String, usize)>>) -> Vec<LabelReference> {
    map.into_iter()
        .flat_map(|(key, values)| {
            values.into_iter().map(move |(path, line)| LabelReference {
                key: key.clone(),
                path,
                line,
            })
        })
        .collect()
}
fn finding(
    code: &str,
    class: FindingClass,
    path: String,
    line: Option<usize>,
    message: String,
    evidence: serde_json::Value,
) -> CheckFinding {
    CheckFinding {
        code: code.into(),
        class,
        path: Some(path),
        line,
        message,
        hint: None,
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn opts(dir: &Path) -> GraphOptions {
        GraphOptions::new(
            Utf8PathBuf::from_path_buf(dir.to_path_buf()).unwrap(),
            "main.tex",
        )
    }
    #[test]
    fn nested_cycle_comments_and_findings() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("main.tex"),
            "\\input{a}\n% \\cite{off}\n\\ref{missing}",
        )
        .unwrap();
        fs::write(
            d.path().join("a.tex"),
            "\\input{b}\n\\label{x}\n\\label{x}\n\\cite{no}",
        )
        .unwrap();
        fs::write(d.path().join("b.tex"), r"\input{a}").unwrap();
        let g = build_dependency_graph(&opts(d.path())).unwrap();
        assert_eq!(
            g.dependencies
                .iter()
                .filter(|n| n.kind != DependencyKind::Bibliography)
                .count(),
            3
        );
        assert!(
            g.report
                .findings
                .iter()
                .any(|f| f.code == "latex.dependency.cycle")
        );
        assert!(
            g.report
                .findings
                .iter()
                .any(|f| f.code == "latex.reference.undefined")
        );
        assert!(
            g.report
                .findings
                .iter()
                .any(|f| f.code == "latex.label.duplicate")
        );
        assert!(
            g.report
                .findings
                .iter()
                .any(|f| f.code == "latex.citation.undefined")
        );
        assert_eq!(g.citations.len(), 1);
    }
    #[test]
    fn deterministic_graph_and_assets() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir(d.path().join("fig")).unwrap();
        fs::write(d.path().join("fig/x.png"), "x").unwrap();
        fs::write(
            d.path().join("main.tex"),
            "\\graphicspath{{fig/}}\n\\includegraphics{x}\n\\includegraphics{x}",
        )
        .unwrap();
        let g = build_dependency_graph(&opts(d.path())).unwrap();
        assert_eq!(g.assets[0].resolved, Some("fig/x.png".into()));
        assert_eq!(g.assets.len(), 2);
        let g2 = build_dependency_graph(&opts(d.path())).unwrap();
        assert_eq!(g, g2);
    }
    #[test]
    fn missing_bib_and_duplicate_keys() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("main.tex"), "\\cite{a}\n\\bibliography{refs}").unwrap();
        fs::write(
            d.path().join("refs.bib"),
            "@article{a, title={x}}\n@book{a, title={y}}",
        )
        .unwrap();
        let g = build_dependency_graph(&opts(d.path())).unwrap();
        assert!(
            g.report
                .findings
                .iter()
                .any(|f| f.code == "latex.bib.key_duplicate")
        );
        assert_eq!(g.citations.len(), 1);
    }
}
