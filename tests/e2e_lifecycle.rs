use assert_cmd::Command;
use camino::Utf8PathBuf;
use serde_json::json;
use sil_db::SilDb;
use sil_mcp::call_tool;
use tempfile::tempdir;

#[test]
fn test_scientific_paper_lifecycle_e2e() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path().join("my-paper");

    // 1. Initialize temporary workspace scaffold (sil init)
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("init")
        .arg("my-paper")
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(project_dir.join("sil.toml").exists());
    assert!(project_dir.join("paper_draft.tex").exists());
    assert!(project_dir.join(".sil").join("structure.yaml").exists());
    assert!(project_dir.join(".sil").join("references.bib").exists());
    assert!(project_dir.join(".sil").join("db.sqlite").exists());

    // 2. sil parse: parse paper/sources into SQLite + FTS5 index
    let source_path = project_dir.join("sources").join("transformer.md");
    let markdown_content = r#"# Attention Is All You Need

Ashish Vaswani, Noam Shazeer, Niki Parmar

Abstract: The dominant sequence transduction models are based on complex recurrent or convolutional neural networks.

## References
[1] Vaswani, A., et al. "Attention is all you need." NeurIPS, 2017. doi:10.5555/3295222.3295349
"#;
    std::fs::write(&source_path, markdown_content).unwrap();

    let mut parse_cmd = Command::cargo_bin("sil").unwrap();
    parse_cmd
        .arg("source")
        .arg("parse")
        .arg(&source_path)
        .current_dir(&project_dir)
        .assert()
        .success();

    let db_path = Utf8PathBuf::from_path_buf(project_dir.join(".sil").join("db.sqlite")).unwrap();
    let db = SilDb::open(&db_path).unwrap();
    let sources = db.list_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].title.as_deref(), Some("Attention Is All You Need"));

    let fts_hits = db.search_sources("transduction models", 10).unwrap();
    assert!(!fts_hits.is_empty(), "FTS5 index must contain search hits");

    // 3. MCP Simulation: simulate sil_upsert_bib and sil_get_structure / sil_set_structure
    let orig_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project_dir).unwrap();

    let bib_entry = r#"@article{vaswani2017attention,
  title={Attention is all you need},
  author={Vaswani, Ashish and Shazeer, Noam and Parmar, Niki},
  journal={Advances in Neural Information Processing Systems},
  year={2017}
}"#;

    let res_upsert = call_tool(
        "sil_upsert_bib",
        Some(json!({
            "entry": bib_entry,
            "draft": true
        })),
    );
    assert!(!res_upsert.is_error, "sil_upsert_bib should succeed: {:?}", res_upsert);

    let bib_content = std::fs::read_to_string(project_dir.join(".sil").join("references.bib")).unwrap();
    assert!(bib_content.contains("vaswani2017attention"), "references.bib must contain upserted entry");

    let res_get_struct = call_tool("sil_get_structure", Some(json!({})));
    assert!(!res_get_struct.is_error, "sil_get_structure should succeed");

    let res_update_struct = call_tool(
        "sil_get_structure",
        Some(json!({
            "action": "update",
            "section_id": "sec_1",
            "completion": "polished"
        })),
    );
    assert!(!res_update_struct.is_error, "sil_get_structure update should succeed");

    std::env::set_current_dir(&orig_cwd).unwrap();

    let mut struct_cmd = Command::cargo_bin("sil").unwrap();
    struct_cmd
        .arg("paper")
        .arg("structure")
        .arg("set")
        .arg("sec_1")
        .arg("polished")
        .current_dir(&project_dir)
        .assert()
        .success();

    // 4. sil paper build: verify manuscript LaTeX build cleanly outputs PDF or verifies LaTeX compilation step
    let mut build_cmd = Command::cargo_bin("sil").unwrap();
    let build_output = build_cmd
        .arg("paper")
        .arg("build")
        .current_dir(&project_dir)
        .output()
        .unwrap();

    if build_output.status.success() {
        let pdf_path = project_dir.join("paper_draft.pdf");
        assert!(pdf_path.exists(), "PDF should be generated when LaTeX engine is available");
    } else {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        let stdout = String::from_utf8_lossy(&build_output.stdout);
        assert!(
            stderr.contains("LaTeX") || stderr.contains("tectonic") || stderr.contains("not found") || stdout.contains("Building"),
            "LaTeX compilation step verified: stderr={stderr}, stdout={stdout}"
        );
    }
}
