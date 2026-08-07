use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_background_hydration_success_upserts_and_preserves_tui_added() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    std::fs::write(
        bib_path.as_std_path(),
        "% [sil: tui-added]\n@article{stub, title={Stub}}\n",
    )
    .unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    app.in_flight_hydration_keys
        .insert("doi:10.1000/182".to_string());

    let official_bib = "@article{stub,\n  title={Official Title},\n  doi={10.1000/182}\n}";
    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/182".to_string(),
            label: "Official Title".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: official_bib.to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    assert!(!app.in_flight_hydration_keys.contains("doi:10.1000/182"));
    let updated_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(updated_content.contains("% [sil: tui-added]"));
    assert!(updated_content.contains("Official Title"));
    assert!(
        app.status_message
            .contains("✓ Official metadata for 'Official Title'")
    );
}

#[test]
fn test_background_hydration_preserves_stub_cite_key() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    std::fs::write(
        bib_path.as_std_path(),
        "% [sil: tui-added]
@article{stub_key, title={Attention Is All You Need}, doi={10.1000/182}}
",
    )
    .unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    app.in_flight_hydration_keys
        .insert("doi:10.1000/182".to_string());

    let official_bib = "@article{Vaswani2017,
  title={Attention Is All You Need},
  author={Vaswani, Ashish},
  doi={10.1000/182}
}";
    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/182".to_string(),
            label: "Attention Is All You Need".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: official_bib.to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    assert!(!app.in_flight_hydration_keys.contains("doi:10.1000/182"));
    let updated_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(updated_content.contains("@article{stub_key,"));
    assert!(updated_content.contains("author = {Vaswani, Ashish}"));
    assert!(!updated_content.contains("Vaswani2017"));
}

#[test]
fn test_background_hydration_failure_warns_and_retains_local() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    let initial_bib = "% [sil: tui-added]\n@article{stub, title={Stub Title}}\n";
    std::fs::write(bib_path.as_std_path(), initial_bib).unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    app.in_flight_hydration_keys
        .insert("doi:10.1000/invalid".to_string());

    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/invalid".to_string(),
            label: "Stub Title".to_string(),
            outcome: HydrationOutcome::Failure {
                reason: "HTTP 404 Not Found".to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    assert!(!app.in_flight_hydration_keys.contains("doi:10.1000/invalid"));
    let content_after = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert_eq!(content_after, initial_bib);
    assert!(
        app.status_message
            .contains("⚠ Metadata fetch failed for 'Stub Title': HTTP 404 Not Found")
    );
}

#[test]
fn test_hydration_deduplication() {
    let mut app = App::new(None);
    let entry = ReferenceEntry {
        id: "ref_dedup".to_string(),
        source_id: "src_1".into(),
        ref_index: 1,
        raw_text: "Ref text".to_string(),
        title: Some("Dedup Title".to_string()),
        authors: None,
        year: None,
        venue: None,
        doi: Some("10.1000/dedup".to_string()),
        arxiv_id: None,
        url: None,
    };

    app.queue_ref_hydration(entry.clone());
    assert!(app.in_flight_hydration_keys.contains("doi:10.1000/dedup"));

    // Attempting second queue for same key should be a no-op
    app.queue_ref_hydration(entry);
    assert_eq!(app.in_flight_hydration_keys.len(), 1);
}

#[test]
fn test_no_fetch_when_no_identifiers() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    std::fs::write(bib_path.as_std_path(), "").unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let empty_entry = ReferenceEntry {
        id: "ref_empty".to_string(),
        source_id: "src_1".into(),
        ref_index: 1,
        raw_text: "Unparseable citation".to_string(),
        title: None,
        authors: None,
        year: None,
        venue: None,
        doi: None,
        arxiv_id: None,
        url: None,
    };

    app.selected_source_references = vec![empty_entry];
    app.selected_viewing_ref_index = 0;
    app.append_selected_viewing_ref_to_bib();

    assert!(app.in_flight_hydration_keys.is_empty());
    assert!(
        app.status_message
            .contains("⚠ No DOI/arXiv/title — cannot hydrate")
    );
    let content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(content.contains("% [sil: tui-added]"));
}

#[test]
fn test_keymap_for_all_modes() {
    let modes = [
        HelpMode::Dashboard,
        HelpMode::SourcesList,
        HelpMode::ReadingSourceMd,
        HelpMode::ViewingSourceRefs,
        HelpMode::ReferencesLeft,
        HelpMode::ReferencesRight,
        HelpMode::PaperDraft,
        HelpMode::Settings,
        HelpMode::ModalPicker,
        HelpMode::ModalAddAuthor,
        HelpMode::ModalAddGrant,
        HelpMode::ModalAddSourceLink,
        HelpMode::ModalRenameSource,
        HelpMode::ConfirmDeleteSource,
        HelpMode::Editing,
        HelpMode::EditingPaper,
        HelpMode::SearchingRefs,
        HelpMode::SearchingBib,
        HelpMode::SearchingViewingRefs,
        HelpMode::JobHistory,
    ];

    for mode in modes {
        let keymap = keymap_for(mode);
        assert!(
            !keymap.is_empty(),
            "Keymap for {:?} should not be empty",
            mode
        );
        assert!(!mode.title().is_empty());
        for (key, action) in keymap {
            assert!(!key.is_empty());
            assert!(!action.is_empty());
        }
    }
}

#[test]
fn test_toggle_help_overlay_and_current_help_mode() {
    let mut app = App::new(None);
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.current_help_mode(), HelpMode::Dashboard);

    // Toggle on ?
    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::HelpOverlay);
    assert_eq!(app.current_help_mode(), HelpMode::Dashboard);

    // Toggle off on any key
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    // Test F1 toggle in Sources view
    app.active_tab = ActiveTab::Sources;
    assert_eq!(app.current_help_mode(), HelpMode::SourcesList);
    app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::HelpOverlay);
    assert_eq!(app.current_help_mode(), HelpMode::SourcesList);

    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_references_title_sort_binding() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::RightSources;
    app.source_references = vec![
        ReferenceEntry {
            id: "r1".to_string(),
            source_id: "s1".into(),
            ref_index: 1,
            raw_text: "Raw Z".to_string(),
            title: Some("Zebra Paper".to_string()),
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        },
        ReferenceEntry {
            id: "r2".to_string(),
            source_id: "s1".into(),
            ref_index: 2,
            raw_text: "Raw A".to_string(),
            title: Some("Alpha Paper".to_string()),
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        },
    ];

    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
    assert_eq!(app.ref_sort_key, RefSortKey::Title);
    assert_eq!(
        app.source_references[0].title.as_deref(),
        Some("Alpha Paper")
    );
    assert_eq!(
        app.source_references[1].title.as_deref(),
        Some("Zebra Paper")
    );
}

#[test]
fn test_hydration_promote_during_flight() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    std::fs::write(
        bib_path.as_std_path(),
        "% [sil: tui-added]\n@article{stub_key, title={Paper Title}, doi={10.1000/race}}\n",
    )
    .unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    app.in_flight_hydration_keys
        .insert("doi:10.1000/race".to_string());

    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::LeftBib;
    app.selected_bib_index = 0;
    app.promote_selected_bib_entry();

    let promoted_before = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(!promoted_before.contains("tui-added"));

    let official_bib = "@article{OfficialKey,\n  title={Paper Title},\n  author={Smith, John},\n  doi={10.1000/race}\n}";
    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/race".to_string(),
            label: "Paper Title".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: official_bib.to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    let updated = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(updated.contains("@article{stub_key,"));
    assert!(updated.contains("author = {Smith, John}"));
    assert!(!updated.contains("tui-added"));
}

#[test]
fn test_hydration_deleted_during_flight() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    std::fs::write(
        bib_path.as_std_path(),
        "% [sil: tui-added]\n@article{stub_key, title={Paper Title}, doi={10.1000/deleted}}\n",
    )
    .unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    app.in_flight_hydration_keys
        .insert("doi:10.1000/deleted".to_string());

    std::fs::write(bib_path.as_std_path(), "").unwrap();

    let official_bib = "@article{OfficialKey, title={Paper Title}, doi={10.1000/deleted}}";
    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/deleted".to_string(),
            label: "Paper Title".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: official_bib.to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    let content_after = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(content_after.is_empty());
    assert_eq!(
        app.status_message,
        "✓ Hydration complete: 1 succeeded, 0 failed"
    );
    assert!(
        app.recent_job_outcomes
            .back()
            .unwrap()
            .detail
            .contains("Skipped hydration for 'Paper Title': entry was deleted")
    );
}

#[test]
fn test_arxiv_only_source_dedup_key() {
    let mut app = App::new(None);
    let doc = SourceDocument {
        id: "src_arxiv".into(),
        path: "2103.12345.pdf".into(),
        filename: "2103.12345.pdf".to_string(),
        kind: sil_core::SourceKind::Pdf,
        parsed: true,
        status: None,
        title: Some("Attention Is All You Need".to_string()),
        authors: None,
        abstract_text: None,
        doi: None,
        year: None,
        venue: None,
        references_text: None,
    };

    app.queue_source_hydration(doc);
    assert!(app.in_flight_hydration_keys.contains("arxiv:2103.12345"));
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn test_hydration_write_failure_status_message() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");
    std::fs::write(
        bib_path.as_std_path(),
        "% [sil: tui-added]\n@article{stub, title={Stub}, doi={10.1000/writeerr}}\n",
    )
    .unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    app.in_flight_hydration_keys
        .insert("doi:10.1000/writeerr".to_string());

    let mut perms = std::fs::metadata(bib_path.as_std_path())
        .unwrap()
        .permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(bib_path.as_std_path(), perms.clone()).unwrap();

    let official_bib = "@article{stub, title={Stub}, author={Tester}, doi={10.1000/writeerr}}";
    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/writeerr".to_string(),
            label: "Stub".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: official_bib.to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    perms.set_readonly(false);
    let _ = std::fs::set_permissions(bib_path.as_std_path(), perms);

    assert_eq!(
        app.status_message,
        "✓ Hydration complete: 1 succeeded, 0 failed"
    );
    assert!(
        app.recent_job_outcomes
            .back()
            .unwrap()
            .detail
            .contains("Error writing references.bib:")
    );
}

#[test]
fn test_poll_multiple_results_in_one_tick_and_batch_drain() {
    let mut app = App::new(None);
    app.in_flight_hydration_keys
        .insert("doi:10.1000/a".to_string());
    app.in_flight_hydration_keys
        .insert("doi:10.1000/b".to_string());
    app.in_flight_hydration_keys
        .insert("doi:10.1000/c".to_string());

    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/a".to_string(),
            label: "Paper A".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: "@article{a, title={Paper A}}".to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/b".to_string(),
            label: "Paper B".to_string(),
            outcome: HydrationOutcome::Failure {
                reason: "HTTP 404".to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.hydration_tx
        .send(HydrationResult {
            dedup_key: "doi:10.1000/c".to_string(),
            label: "Paper C".to_string(),
            outcome: HydrationOutcome::Success {
                official_bib: "@article{c, title={Paper C}}".to_string(),
            },
            duration_ms: None,
        })
        .unwrap();

    app.poll_background_hydration();

    assert!(app.in_flight_hydration_keys.is_empty());
    assert_eq!(app.hydration_batch_succeeded, 2);
    assert_eq!(app.hydration_batch_failed, 1);
    assert_eq!(app.recent_job_outcomes.len(), 3);
    assert_eq!(
        app.status_message,
        "✓ Hydration complete: 2 succeeded, 1 failed"
    );
}

#[test]
fn test_already_hydrating_dedup_and_status() {
    let mut app = App::new(None);
    let entry = sil_core::ReferenceEntry {
        id: "ref_1".to_string(),
        source_id: "src_1".into(),
        ref_index: 1,
        raw_text: "Test Reference".to_string(),
        title: Some("Duplicate Test Paper".to_string()),
        authors: None,
        year: None,
        venue: None,
        doi: Some("10.1000/dup".to_string()),
        arxiv_id: None,
        url: None,
    };

    app.queue_ref_hydration(entry.clone());
    assert!(app.in_flight_hydration_keys.contains("doi:10.1000/dup"));
    assert_eq!(app.status_message, "⏳ Hydrating (1 in flight)...");

    // Request again while in flight
    app.queue_ref_hydration(entry);
    assert_eq!(
        app.status_message,
        "already hydrating 'Duplicate Test Paper'..."
    );
    assert_eq!(app.in_flight_hydration_keys.len(), 1);
}

#[test]
fn test_recent_job_outcomes_bounded_to_20() {
    let mut app = App::new(None);
    for i in 0..25 {
        app.in_flight_hydration_keys
            .insert(format!("doi:10.1000/{i}"));
        app.hydration_tx
            .send(HydrationResult {
                dedup_key: format!("doi:10.1000/{i}"),
                label: format!("Paper {i}"),
                outcome: HydrationOutcome::Success {
                    official_bib: format!("@article{{p{i}, title={{Paper {i}}}}}"),
                },
                duration_ms: None,
            })
            .unwrap();
    }

    app.poll_background_hydration();

    assert_eq!(app.recent_job_outcomes.len(), 20);
    assert_eq!(app.recent_job_outcomes.front().unwrap().label, "Paper 5");
    assert_eq!(app.recent_job_outcomes.back().unwrap().label, "Paper 24");
}

#[test]
fn test_classify_source_input() {
    assert_eq!(
        classify_source_input("10.1038/s41586-020-2649-2"),
        SourceInputKind::Doi
    );
    assert_eq!(
        classify_source_input("doi:10.1145/1234567"),
        SourceInputKind::Doi
    );
    assert_eq!(
        classify_source_input("https://doi.org/10.1145/1234567"),
        SourceInputKind::Doi
    );

    assert_eq!(classify_source_input("2103.12345"), SourceInputKind::Arxiv);
    assert_eq!(
        classify_source_input("arXiv:2103.12345v1"),
        SourceInputKind::Arxiv
    );
    assert_eq!(
        classify_source_input("https://arxiv.org/abs/2103.12345"),
        SourceInputKind::Arxiv
    );

    assert_eq!(
        classify_source_input("https://example.com/paper.pdf"),
        SourceInputKind::Url
    );
    assert_eq!(
        classify_source_input("http://site.org/resource"),
        SourceInputKind::Url
    );

    assert_eq!(
        classify_source_input("paper_notes.md"),
        SourceInputKind::Filename
    );
    assert_eq!(classify_source_input(""), SourceInputKind::Filename);
}

#[test]
fn test_sources_reload_action_key_r() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let mut app = App::new(Some(root));

    app.active_tab = ActiveTab::Sources;
    app.status_message = "Initial status".to_string();

    // Press 'R' key in Sources tab
    app.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::empty()));
    assert_eq!(app.status_message, "✓ Reloaded sources");

    // Press Shift+'r' in Sources tab
    app.status_message = "Initial status".to_string();
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::SHIFT));
    assert_eq!(app.status_message, "✓ Reloaded sources");
}

#[test]
fn test_sources_parse_keymap() {
    let keymap = keymap_for(HelpMode::SourcesList);
    let parse_entry = keymap.iter().find(|(key, _)| *key == "e / E");
    assert!(
        parse_entry.is_some(),
        "Keymap for SourcesList missing 'e / E'"
    );
}

#[test]
fn test_sources_parse_already_parsed_status() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Sources;
    let mut doc = SourceDocument::new(camino::Utf8PathBuf::from("test.txt"));
    doc.parsed = true;
    app.sources.push(doc);

    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
    assert_eq!(
        app.status_message,
        "ℹ Source is already parsed (use 'E' / Shift+E to re-parse)"
    );
    assert!(app.in_flight_parse_ids.is_empty());
}

#[test]
fn test_sources_parse_queueing_normal_and_force() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    let sources_dir = root.join("sources");
    std::fs::create_dir_all(sources_dir.as_std_path()).unwrap();
    let file_path = sources_dir.join("sample.txt");
    std::fs::write(
        file_path.as_std_path(),
        "Title: Sample Paper\nAbstract: Test abstract\n\nReferences:\n[1] A. Author, Sample Reference, 2024.",
    )
    .unwrap();

    let mut app = App::new(Some(root.clone()));
    app.active_tab = ActiveTab::Sources;
    app.reload_sources();
    assert_eq!(app.sources.len(), 1);
    assert!(!app.sources[0].parsed);

    // Queue normal parse with 'e'
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
    assert!(app.in_flight_parse_ids.contains(&app.sources[0].id));
    assert!(app.status_message.starts_with("⏳ Parsing source"));

    // Wait for background parse thread to complete
    for _ in 0..50 {
        app.poll_background_hydration();
        if app.in_flight_parse_ids.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(app.in_flight_parse_ids.is_empty());
    assert!(app.status_message.starts_with("✓ Parsed source"));
    assert!(app.sources[0].parsed);

    // Pressing 'e' now should inform user that it's already parsed
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
    assert_eq!(
        app.status_message,
        "ℹ Source is already parsed (use 'E' / Shift+E to re-parse)"
    );

    // Pressing Shift+E ('E') should force re-parse
    app.handle_key(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
    assert!(app.in_flight_parse_ids.contains(&app.sources[0].id));
    assert!(app.status_message.starts_with("⏳ Parsing source"));

    for _ in 0..50 {
        app.poll_background_hydration();
        if app.in_flight_parse_ids.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(app.in_flight_parse_ids.is_empty());
    assert!(app.status_message.starts_with("✓ Parsed source"));
}

#[test]
fn test_sources_parse_failure_status() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();

    let mut app = App::new(Some(root));
    app.active_tab = ActiveTab::Sources;
    let doc = SourceDocument::new(Utf8PathBuf::from("/nonexistent/file.pdf"));
    app.sources.push(doc);

    // Queue force parse on non-existent file
    app.handle_key(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
    assert!(!app.in_flight_parse_ids.is_empty());

    for _ in 0..50 {
        app.poll_background_hydration();
        if app.in_flight_parse_ids.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(app.in_flight_parse_ids.is_empty());
    assert!(app.status_message.starts_with("⚠ Failed parsing source"));
}
