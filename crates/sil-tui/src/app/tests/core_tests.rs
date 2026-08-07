use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn app_initialization() {
    let app = App::new(None);
    assert_eq!(app.active_tab, ActiveTab::Dashboard);
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_tab_navigation() {
    let mut app = App::new(None);
    assert_eq!(app.active_tab, ActiveTab::Dashboard);

    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::Sources);

    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::References);

    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::PaperDraft);

    app.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::Settings);

    app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::Dashboard);
}

#[test]
fn test_references_tab_navigation() {
    let mut app = App::new(None);

    // Go to References
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::References);

    // Default pane is RightSources
    assert_eq!(app.active_ref_pane, RefPane::RightSources);

    // Tab toggles pane
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
    assert_eq!(app.active_ref_pane, RefPane::LeftBib);

    // Tab again toggles pane back
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
    assert_eq!(app.active_ref_pane, RefPane::RightSources);
}

#[test]
fn test_references_marking_and_searching() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::References;
    app.source_references = vec![
        sil_core::ReferenceEntry {
            id: "ref1".to_string(),
            source_id: "doc1".into(),
            ref_index: 1,
            raw_text: "Deep learning".to_string(),
            title: None,
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        },
        sil_core::ReferenceEntry {
            id: "ref2".to_string(),
            source_id: "doc2".into(),
            ref_index: 2,
            raw_text: "Transformer models".to_string(),
            title: None,
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        },
    ];

    app.selected_source_ref_index = 0;
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(app.marked_ref_ids.contains("ref1"));

    app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::SearchingRefs);

    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    app.selected_source_ref_index = 0;
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(app.marked_ref_ids.contains("ref2"));
}

#[test]
fn test_enums_and_titles() {
    assert_eq!(ActiveTab::ALL.len(), 5);
    assert_eq!(ActiveTab::Dashboard.title(), "1. Dashboard");
    assert_eq!(ActiveTab::Sources.title(), "2. Sources");
    assert_eq!(ActiveTab::References.title(), "3. References");
    assert_eq!(ActiveTab::PaperDraft.title(), "4. Paper Draft");
    assert_eq!(ActiveTab::Settings.title(), "5. Settings");

    assert_eq!(GlobalField::ALL.len(), 9);
    assert_eq!(LocalField::ALL.len(), 4);
    assert_eq!(RagField::ALL.len(), 9);
}

#[test]
fn test_resolve_onnx_from_dir() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

    // Non-dir string
    assert_eq!(resolve_onnx_from_dir("/no/such/dir"), "/no/such/dir");

    // Dir without onnx
    assert_eq!(resolve_onnx_from_dir(dir_path.as_str()), dir_path.as_str());

    // Dir with onnx file
    let onnx_file = dir_path.join("model.onnx");
    std::fs::write(onnx_file.as_std_path(), b"onnx").unwrap();
    let resolved = resolve_onnx_from_dir(dir_path.as_str());
    assert!(resolved.ends_with("model.onnx"));
}

#[test]
fn test_app_with_project_root_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

    // Create paper_draft.tex
    let tex_content = "\\section{Intro}\nHello world\n";
    std::fs::write(root.join("paper_draft.tex").as_std_path(), tex_content).unwrap();

    // Create references.bib
    let bib_content =
        "@misc{key1,\n  title = {Paper One},\n}\n@article{key2,\n  title = {Paper Two},\n}\n";
    std::fs::write(root.join("references.bib").as_std_path(), bib_content).unwrap();

    // Create sources dir with md file
    let sources_dir = root.join("sources");
    std::fs::create_dir_all(sources_dir.as_std_path()).unwrap();
    std::fs::write(sources_dir.join("readme.md").as_std_path(), "ignore me").unwrap();
    std::fs::write(
        sources_dir.join("source1.md").as_std_path(),
        "# Source 1 Content",
    )
    .unwrap();

    let app = App::new(Some(root.clone()));
    assert_eq!(app.paper_draft_content, tex_content);
    assert_eq!(app.bib_file_entries.len(), 2);
    assert_eq!(app.sources.len(), 1);
    assert_eq!(app.sources[0].filename, "source1.md");

    // Test fetch_source_markdown_content
    let content = app.fetch_source_markdown_content(&app.sources[0]);
    assert_eq!(content, "# Source 1 Content");
}

#[test]
fn test_normal_mode_navigation_and_shortcuts() {
    let mut app = App::new(None);

    // Ctrl+s saves
    app.dirty = true;
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    assert!(!app.dirty);

    // esc / q quits
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
    assert!(app.should_quit);

    app.should_quit = false;
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert!(app.should_quit);

    // BackTab
    app.should_quit = false;
    app.active_tab = ActiveTab::Dashboard;
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::Settings);

    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::PaperDraft);
}

#[test]
fn test_dashboard_up_down_keys() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Dashboard;
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.active_tab, ActiveTab::Dashboard);
}

#[test]
fn test_sources_tab_actions() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Sources;
    app.sources = vec![SourceDocument::new(Utf8PathBuf::from("test.md"))];

    // Read source markdown
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert!(app.reading_md_content.is_some());

    // Exit reader with Esc
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.reading_md_content.is_none());

    // Add source link mode
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ModalAddSourceLink);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

    // Rename source mode
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ModalRenameSource);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

    // Confirm delete source mode
    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ConfirmDeleteSource);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));

    // View source references
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ViewingSourceRefs);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
}

#[test]
fn test_editing_all_global_settings_fields() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Settings;

    // Iterate over setting items and test editing fields
    let items = app.setting_items();
    for (idx, item) in items.iter().enumerate() {
        if let SettingItem::Global(field) = item {
            app.selected_setting_index = idx;
            app.start_editing_selected_field();
            assert_eq!(app.input_mode, InputMode::Editing);

            app.input_buffer = match field {
                GlobalField::AuthorName => "New Author".to_string(),
                GlobalField::AuthorEmail => "author@test.com".to_string(),
                GlobalField::AuthorAffiliation => "Test Uni".to_string(),
                GlobalField::AuthorOrcid => "0000-0002".to_string(),
                GlobalField::GrantFunder => "DOE".to_string(),
                GlobalField::GrantNumber => "G-100".to_string(),
                GlobalField::GrantAck => "Thanks DOE".to_string(),
                GlobalField::Engine => "pdflatex".to_string(),
                GlobalField::Template => "neurips".to_string(),
            };

            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
            assert_eq!(app.input_mode, InputMode::Normal);
        }
    }

    assert_eq!(app.global_settings.author.name, "New Author");
    assert_eq!(app.global_settings.author.email, "author@test.com");
    assert_eq!(app.global_settings.author.affiliation, "Test Uni");
    assert_eq!(
        app.global_settings.author.orcid,
        Some("0000-0002".to_string())
    );
    assert_eq!(app.global_settings.default_grant.funder, "DOE");
    assert_eq!(app.global_settings.default_grant.grant_number, "G-100");
    assert_eq!(
        app.global_settings.default_grant.acknowledgment,
        "Thanks DOE"
    );
    assert_eq!(app.global_settings.default_latex_engine, "pdflatex");
    assert_eq!(app.global_settings.default_template, "neurips");
}

#[test]
fn test_editing_all_rag_settings_fields() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Settings;

    let items = app.setting_items();
    for (idx, item) in items.iter().enumerate() {
        if let SettingItem::Rag(field) = item {
            app.selected_setting_index = idx;
            app.start_editing_selected_field();
            assert_eq!(app.input_mode, InputMode::Editing);

            app.input_buffer = match field {
                RagField::EmbedderPath => "/path/to/embedder.onnx".to_string(),
                RagField::RerankerPath => "/path/to/reranker.onnx".to_string(),
                RagField::ModelsDir => "/path/to/models".to_string(),
                RagField::CacheDir => "/path/to/cache".to_string(),
                RagField::XbergCacheDir => "/path/to/xberg_cache".to_string(),
                RagField::ExecutionProvider => "cuda".to_string(),
                RagField::NumThreads => "16".to_string(),
                RagField::ParentChunkSize => "2000".to_string(),
                RagField::ChildChunkSize => "500".to_string(),
            };

            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
            assert_eq!(app.input_mode, InputMode::Normal);
        }
    }

    assert_eq!(
        app.global_settings.rag.onnx_embedder_path,
        Some(Utf8PathBuf::from("/path/to/embedder.onnx"))
    );
    assert_eq!(
        app.global_settings.rag.onnx_reranker_path,
        Some(Utf8PathBuf::from("/path/to/reranker.onnx"))
    );
    assert_eq!(
        app.global_settings.rag.onnx_models_dir,
        Some(Utf8PathBuf::from("/path/to/models"))
    );
    assert_eq!(
        app.global_settings.rag.model_cache_dir,
        Utf8PathBuf::from("/path/to/cache")
    );
    assert_eq!(app.global_settings.rag.execution_provider, "cuda");
    assert_eq!(app.global_settings.rag.num_threads, 16);
    assert_eq!(app.global_settings.rag.parent_chunk_size, 2000);
    assert_eq!(app.global_settings.rag.child_chunk_size, 500);
}

#[test]
fn test_editing_local_settings_fields() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Settings;

    let items = app.setting_items();
    for (idx, item) in items.iter().enumerate() {
        match item {
            SettingItem::LocalTitle => {
                app.selected_setting_index = idx;
                app.start_editing_selected_field();
                app.input_buffer = "My Great Paper".to_string();
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
            }
            SettingItem::LocalNotes => {
                app.selected_setting_index = idx;
                app.start_editing_selected_field();
                app.input_buffer = "Important research notes".to_string();
                app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
            }
            _ => {}
        }
    }

    assert_eq!(app.local_settings.title, "My Great Paper");
    assert_eq!(app.local_settings.notes, "Important research notes");
}

#[test]
fn test_modal_add_author_and_grant_workflows() {
    let mut app = App::new(None);
    app.cache = SettingsCache::default();
    app.local_settings = LocalSettings::default();

    // Add author modal
    app.input_mode = InputMode::ModalAddAuthor;
    app.modal_field_index = 0;

    // Type name
    app.handle_key(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
    assert_eq!(app.new_author.name, "Bob");

    // Tab to email
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
    assert_eq!(app.modal_field_index, 1);
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('@'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()));
    assert_eq!(app.new_author.email, "b@m");

    // Enter submits author
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.cache.co_authors.len(), 1);
    assert_eq!(app.local_settings.co_authors.len(), 1);

    // Add grant modal
    app.input_mode = InputMode::ModalAddGrant;
    app.modal_field_index = 0;

    app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty()));
    assert_eq!(app.new_grant.funder, "NIH");

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.cache.grants.len(), 1);
    assert_eq!(app.local_settings.grants.len(), 1);
}

#[test]
fn test_modal_add_source_link_and_rename_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let mut app = App::new(Some(root.clone()));

    // Add source link modal
    app.active_tab = ActiveTab::Sources;
    app.input_mode = InputMode::ModalAddSourceLink;
    app.new_source_link_buffer = "https://example.com/paper.pdf".to_string();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(
        app.in_flight_fetch_targets
            .contains("https://example.com/paper.pdf")
    );

    // Push a source to test rename and delete
    let mut doc = sil_core::SourceDocument::new(camino::Utf8PathBuf::from("sources/paper.pdf"));
    doc.title = Some("Original Title".to_string());
    app.sources.push(doc);

    // Rename source modal
    app.selected_source_index = 0;
    app.input_mode = InputMode::ModalRenameSource;
    app.rename_source_buffer = "Renamed Title".to_string();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.sources[0].title, Some("Renamed Title".to_string()));

    // Confirm delete source modal
    app.input_mode = InputMode::ConfirmDeleteSource;
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
    assert!(app.sources.is_empty());
}

#[test]
fn test_paper_draft_editing_and_scrolling() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::PaperDraft;
    app.paper_draft_content = "\\section{Intro}\nInitial content".to_string();
    app.paper_sections = sil_latex::split_tex_sections(&app.paper_draft_content);
    app.paper_section_index = 0;

    // Edit paper section
    app.start_editing_selected_field();
    assert_eq!(app.input_mode, InputMode::EditingPaper);
    app.paper_edit_buffer = "Updated intro section body".to_string();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(
        app.paper_draft_content
            .contains("Updated intro section body")
    );

    // PageUp / PageDown scrolling
    app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
    assert_eq!(app.paper_scroll_offset, 5);
    app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
    assert_eq!(app.paper_scroll_offset, 0);
}

#[test]
fn test_viewing_source_refs_sorting() {
    let mut app = App::new(None);
    app.input_mode = InputMode::ViewingSourceRefs;
    app.selected_source_references = vec![
        ReferenceEntry {
            id: "1".to_string(),
            source_id: "s1".into(),
            ref_index: 2,
            raw_text: "Ref 2".to_string(),
            title: None,
            authors: None,
            year: Some(2020),
            venue: Some("NeurIPS".to_string()),
            doi: None,
            arxiv_id: None,
            url: None,
        },
        ReferenceEntry {
            id: "2".to_string(),
            source_id: "s2".into(),
            ref_index: 1,
            raw_text: "Ref 1".to_string(),
            title: None,
            authors: None,
            year: Some(2024),
            venue: Some("ICML".to_string()),
            doi: None,
            arxiv_id: None,
            url: None,
        },
    ];

    // Sort by Year
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
    assert_eq!(app.ref_sort_key, RefSortKey::Year);
    assert_eq!(app.selected_source_references[0].year, Some(2024));

    // Sort by Venue
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
    assert_eq!(app.ref_sort_key, RefSortKey::Venue);
    assert_eq!(
        app.selected_source_references[0].venue,
        Some("ICML".to_string())
    );

    // Sort by Index
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
    assert_eq!(app.ref_sort_key, RefSortKey::Index);
    assert_eq!(app.selected_source_references[0].ref_index, 1);

    // Sort by Source
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
    assert_eq!(app.ref_sort_key, RefSortKey::Source);
    assert_eq!(app.selected_source_references[0].source_id.as_str(), "s1");

    // Esc closes viewer
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_save_all_in_project() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let sil_dir = root.join(".sil");
    std::fs::create_dir_all(sil_dir.as_std_path()).unwrap();

    let config_path = sil_dir.join("config.yaml");
    let initial_cfg = Config::default();
    std::fs::write(config_path.as_std_path(), initial_cfg.to_yaml().unwrap()).unwrap();

    let mut app = App::new(Some(root.clone()));
    app.local_settings.title = "Saved Paper Title".to_string();
    app.paper_draft_content = "\\section{Main}\nSaved content\n".to_string();
    app.dirty = true;

    app.save_all();
    assert!(!app.dirty);
    assert!(app.status_message.contains("✓"));

    let reloaded_cfg = Config::load(&config_path).unwrap();
    assert_eq!(reloaded_cfg.project.title, "Saved Paper Title");

    let draft_path = root.join("paper_draft.tex");
    let saved_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(saved_tex, "\\section{Main}\nSaved content\n");
}

#[test]
fn test_left_bib_search_and_filtering() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::LeftBib;

    app.bib_file_entries = vec![
        "@article{attn, title={Attention is All You Need}}".to_string(),
        "@misc{resnet, title={Deep Residual Learning}}".to_string(),
    ];

    // Pressing '/' in LeftBib enters SearchingBib
    app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::SearchingBib);

    // Type query 'attn'
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));

    assert_eq!(app.bib_search_query, "attn");
    assert_eq!(app.filtered_bib_entries().len(), 1);
    assert!(app.filtered_bib_entries()[0].contains("attn"));

    // Enter exits SearchingBib mode
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    // Pressing Esc in Normal mode with active filter clears the query
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.bib_search_query, "");
    assert_eq!(app.filtered_bib_entries().len(), 2);
    assert!(!app.should_quit);
}

#[test]
fn test_references_tab_right_pane_sorting() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::RightSources;

    app.source_references = vec![
        ReferenceEntry {
            id: "ref_a".to_string(),
            source_id: "src_z".into(),
            ref_index: 2,
            raw_text: "Ref A".to_string(),
            title: Some("Paper A".to_string()),
            authors: Some("Author A".to_string()),
            year: Some(2020),
            venue: Some("NeurIPS".to_string()),
            doi: None,
            arxiv_id: None,
            url: None,
        },
        ReferenceEntry {
            id: "ref_b".to_string(),
            source_id: "src_a".into(),
            ref_index: 1,
            raw_text: "Ref B".to_string(),
            title: Some("Paper B".to_string()),
            authors: Some("Author B".to_string()),
            year: Some(2024),
            venue: Some("ICML".to_string()),
            doi: None,
            arxiv_id: None,
            url: None,
        },
    ];

    // Sort by year ('y')
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
    assert_eq!(app.source_references[0].year, Some(2024));

    // Sort by venue ('v')
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
    assert_eq!(app.source_references[0].venue, Some("ICML".to_string()));

    // Sort by source_id ('s')
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()));
    assert_eq!(app.source_references[0].source_id.as_str(), "src_a");

    // Sort by index ('i')
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()));
    assert_eq!(app.source_references[0].ref_index, 1);
}

#[test]
fn test_viewing_source_refs_navigation_and_scrolling() {
    let mut app = App::new(None);
    app.input_mode = InputMode::ViewingSourceRefs;
    app.selected_source_references = (1..=10)
        .map(|idx| ReferenceEntry {
            id: format!("ref_{idx}"),
            source_id: "doc1.pdf".into(),
            ref_index: idx,
            raw_text: format!("Reference item {idx}"),
            title: Some(format!("Title {idx}")),
            authors: Some(format!("Author {idx}")),
            year: Some(2010 + idx as i32),
            venue: Some("Conf".to_string()),
            doi: None,
            arxiv_id: None,
            url: None,
        })
        .collect();
    app.selected_viewing_ref_index = 0;

    // Down / 'j' navigation
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 2);

    // Up / 'k' navigation
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 1);

    // PageDown & PageUp
    app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 6);

    app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 1);

    // End & Home
    app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 9);

    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
    assert_eq!(app.selected_viewing_ref_index, 0);
}

#[test]
fn test_viewing_source_refs_sorting_and_filtering() {
    let mut app = App::new(None);
    app.input_mode = InputMode::ViewingSourceRefs;
    app.selected_source_references = vec![
        ReferenceEntry {
            id: "ref_1".to_string(),
            source_id: "src1".into(),
            ref_index: 1,
            raw_text: "Attention is All You Need".to_string(),
            title: Some("Attention is All You Need".to_string()),
            authors: Some("Vaswani".to_string()),
            year: Some(2017),
            venue: Some("NeurIPS".to_string()),
            doi: Some("10.1000/1".to_string()),
            arxiv_id: None,
            url: None,
        },
        ReferenceEntry {
            id: "ref_2".to_string(),
            source_id: "src2".into(),
            ref_index: 2,
            raw_text: "Deep Residual Learning".to_string(),
            title: Some("Deep Residual Learning".to_string()),
            authors: Some("He".to_string()),
            year: Some(2016),
            venue: Some("CVPR".to_string()),
            doi: None,
            arxiv_id: None,
            url: None,
        },
    ];

    // Sort by Year ('y')
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
    assert_eq!(app.selected_source_references[0].year, Some(2017));

    // Sort by Title ('t')
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()));
    assert_eq!(
        app.selected_source_references[0].title.as_deref(),
        Some("Attention is All You Need")
    );

    // Enter search mode with '/'
    app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::SearchingViewingRefs);

    // Type 'He'
    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
    assert_eq!(app.filtered_viewing_source_references().len(), 1);
    assert_eq!(
        app.filtered_viewing_source_references()[0]
            .authors
            .as_deref(),
        Some("He")
    );

    // Esc exits search mode
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ViewingSourceRefs);
}

#[test]
fn test_viewing_source_refs_bibtex_append_and_delete() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    std::fs::write(project_path.join("references.bib").as_std_path(), "").unwrap();

    let mut app = App::new(Some(project_path.clone()));
    app.input_mode = InputMode::ViewingSourceRefs;
    app.selected_source_references = vec![ReferenceEntry {
        id: "ref_1".to_string(),
        source_id: "src1".into(),
        ref_index: 1,
        raw_text: "Attention is All You Need".to_string(),
        title: Some("Attention is All You Need".to_string()),
        authors: Some("Vaswani".to_string()),
        year: Some(2017),
        venue: Some("NeurIPS".to_string()),
        doi: None,
        arxiv_id: None,
        url: None,
    }];
    app.selected_viewing_ref_index = 0;

    // Append selected ref to bib via 'c'
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
    let bib_content =
        std::fs::read_to_string(project_path.join("references.bib").as_std_path()).unwrap();
    assert!(bib_content.to_lowercase().contains("attention") || bib_content.contains("Vaswani"));
    assert_eq!(app.bib_file_entries.len(), 1);

    // Delete bib entry via delete_selected_bib_entry
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::LeftBib;
    app.selected_bib_index = 0;
    app.delete_selected_bib_entry();

    let updated_bib =
        std::fs::read_to_string(project_path.join("references.bib").as_std_path()).unwrap();
    assert!(updated_bib.is_empty());
    assert_eq!(app.bib_file_entries.len(), 0);
}

#[test]
fn test_load_bib_entries_with_comments_and_indentation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    let bib_text = r#"
# Top level comment
@article{entry1,
  title={Paper 1}
}

  @inproceedings{entry2,
  title={Paper 2}
}
"#;
    std::fs::write(project_path.join("references.bib").as_std_path(), bib_text).unwrap();

    let mut app = App::new(Some(project_path));
    app.load_project_references_bib();
    assert_eq!(app.bib_file_entries.len(), 2);
    assert!(app.bib_file_entries[0].contains("entry1"));
    assert!(app.bib_file_entries[1].contains("entry2"));
}

#[test]
fn test_sources_tab_append_selected_to_bib() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    std::fs::write(project_path.join("references.bib").as_std_path(), "").unwrap();

    let mut app = App::new(Some(project_path.clone()));
    app.active_tab = ActiveTab::Sources;
    let mut doc = SourceDocument::new("test_paper.pdf".into());
    doc.title = Some("Deep Learning Advances".into());
    doc.authors = Some("Alice Smith".into());
    app.sources = vec![doc];
    app.selected_source_index = 0;

    // Press 'b' to append selected source to references.bib
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));

    // Since test_paper.pdf has no DOI or Crossref hit, it warns
    assert!(app.status_message.contains("⚠") || app.status_message.contains("✓"));
}

#[test]
fn test_references_similarity_sorting_and_filtering() {
    let mut app = App::new(None);
    let ref1 = ReferenceEntry {
        id: "ref_1".to_string(),
        source_id: "src".into(),
        ref_index: 1,
        raw_text: "Low similarity ref".to_string(),
        title: Some("Low similarity ref".to_string()),
        authors: None,
        year: Some(2020),
        venue: None,
        doi: None,
        arxiv_id: None,
        url: None,
    };
    let ref2 = ReferenceEntry {
        id: "ref_2".to_string(),
        source_id: "src".into(),
        ref_index: 2,
        raw_text: "High similarity ref".to_string(),
        title: Some("High similarity ref".to_string()),
        authors: None,
        year: Some(2021),
        venue: None,
        doi: None,
        arxiv_id: None,
        url: None,
    };

    app.source_references = vec![ref1, ref2];
    app.draft_ref_similarities.insert("ref_1".to_string(), 0.25);
    app.draft_ref_similarities.insert("ref_2".to_string(), 0.95);

    // Sort by similarity via 'm'
    app.active_tab = ActiveTab::References;
    app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()));
    assert_eq!(app.ref_sort_key, RefSortKey::Similarity);

    let filtered = app.filtered_source_references();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].id, "ref_2"); // highest score first
    assert_eq!(filtered[1].id, "ref_1");

    // Filter by min similarity score threshold = 0.5
    app.min_similarity_filter = Some(0.5);
    let filtered_threshold = app.filtered_source_references();
    assert_eq!(filtered_threshold.len(), 1);
    assert_eq!(filtered_threshold[0].id, "ref_2");
}

#[test]
fn test_tui_added_bib_entry_marking_and_promote() {
    use camino::Utf8Path;
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let bib_path = root.join("references.bib");

    let mut app = App::new(Some(root.to_path_buf()));
    let ref_entry = ReferenceEntry {
        id: "ref_test".to_string(),
        source_id: "src_test".into(),
        ref_index: 1,
        raw_text: "Sample Raw Reference Text".to_string(),
        title: Some("Sample Reference Title".to_string()),
        authors: Some("Author A".to_string()),
        year: Some(2024),
        venue: None,
        doi: None,
        arxiv_id: None,
        url: None,
    };
    app.source_references = vec![ref_entry];
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::RightSources;
    app.selected_source_ref_index = 0;

    // Paste/add reference to references.bib using 'p'
    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()));

    let bib_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(bib_content.contains("% [sil: tui-added]"));
    assert!(bib_content.contains("@"));

    // Switch to LeftBib pane and promote using 'P'
    app.active_ref_pane = RefPane::LeftBib;
    app.selected_bib_index = 0;
    app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));

    let promoted_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(!promoted_content.contains("tui-added"));
    assert!(promoted_content.contains("@"));
}
