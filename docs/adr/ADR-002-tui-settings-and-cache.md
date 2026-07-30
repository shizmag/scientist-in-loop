# ADR-002: TUI Settings & Co-Author Cache (`sil-tui`)

## Context

`scientist-in-loop` projects require metadata for author details, grant requisites, co-authors, and article-specific configuration.
Authors routinely work across multiple articles and papers where:
1. Global user details (primary author name, default grant requisites, email, affiliation, ORCID) remain consistent across all articles.
2. Local settings (article title, specific co-authors, article-specific grants, template selection) vary per project.
3. Co-authors and grants are frequently reused across articles, so entering them repeatedly creates friction.

## Decision

We introduce a dedicated TUI crate `crates/sil-tui` using `ratatui` and `crossterm`, supported by settings storage and cache logic in `crates/sil-core`:

1. **Global Settings (`~/.config/sil/settings.yaml`)**: Stores primary author information, default grant requisites, default LaTeX engine, and default template.
2. **Local Settings (`.sil/config.yaml`)**: Stores project-level metadata (article title, active co-authors list, active grant requisites, project notes) alongside existing project settings.
3. **Settings Cache (`~/.config/sil/cache.yaml`)**: Automatically caches co-authors and grant requisites encountered across any paper. Provides fast selection modals and autocomplete in the TUI for populating local project co-authors.
4. **Interactive TUI (`sil settings` / `sil tui`)**: A styled `ratatui` application with tabbed navigation (Global Settings, Local Settings, Co-Authors Cache, Grants Cache), inline field editing, cached co-author picker, and persistent save actions (`Ctrl+S`/`s`).

## Consequences

- Authors can manage global, local, and historical co-author settings seamlessly inside a single interactive interface.
- Co-author and grant details are remembered automatically, eliminating repetitive manual input across new projects.
- `sil-core` remains the single source of truth for config and settings data structures, keeping `sil-tui` focused on UI presentation.
