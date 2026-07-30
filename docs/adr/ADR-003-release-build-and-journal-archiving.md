# ADR-003: Release Build and Autonomous Journal Archiving

## Context

Human researchers and AI agents collaborate on paper drafts using `paper_draft.tex` and internal note blocks (`# -- X -- #`). When building for publication submission, all draft-only notes must be excluded, target venue templates applied, and an autonomous zip archive containing all required source and binary dependencies generated for journal upload.

## Decision

1. **CLI Interface**:
   - `sil build release` (or `sil build realese`) replaces the former `--release` flag as the primary subcommand syntax for release compilation (while preserving `--release` for backwards compatibility).
   - `sil build` without `release` runs in draft mode.

2. **Draft Note Isolation**:
   - `# -- X -- #` (and `% # -- X -- #`) blocks are retained in draft builds but automatically stripped during `sil build release` and template application.

3. **Autonomous Submission Archive**:
   - `sil build release` packages the generated manuscript `.tex`, output `.pdf`, bibliography databases (`.bib`), class/style/BST files (`.cls`, `.sty`, `.bst`), and referenced figure assets into `submission_<template>.zip`.

## Consequences

- Clean separation between draft iteration and publication release.
- Self-contained submission ZIP ready to be submitted to journal web portals.
