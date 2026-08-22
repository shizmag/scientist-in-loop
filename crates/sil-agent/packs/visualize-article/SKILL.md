---
id: visualize-article
name: visualize-article
version: 1.0.0
title: Visualize Article
description: Generate detailed English prompts for academic figures such as frameworks, pipelines, architectures, comparisons, and data grids.
license: MIT
triggers:
  - figure
  - figures
  - visualization
  - visualise
  - visualize
  - diagram
  - pipeline
  - architecture
required_capabilities:
  - network
  - resources
inputs:
  - paper_draft.tex
outputs:
  - figures/
permissions:
  - "read:manuscript"
  - "read:figures"
  - "network:external_image_provider"
verification: check_figures
external_data_flow:
  destination: external_image_provider
  data_classes:
    - manuscript
    - figures
  consent_required: true
  disclosure: Sends user-approved manuscript and figure details to an external image provider only when the host obtains consent.
---
# Visualize Article

Generate detailed English prompts for academic figures such as frameworks, pipelines, architectures, comparisons, and data grids.

This skill generates prompts for an external image provider. It does not call an image API, render an image, or produce a finished figure.

## Inputs

- Figure purpose and intended audience
- Source claims, measurements, and labels from the manuscript
- Required dimensions, typography, and conference constraints

## Output

Return one precise prompt with layout, hierarchy, labels, visual encoding, and an explicit list of source facts. Flag missing facts instead of inventing them.

## Workflow: Inspect -> Propose -> Modify -> Verify

### 1. Inspect
- Inspect manuscript prose (`paper_draft.tex`), structure (`.sil/structure.yaml`), and existing figures in `figures/`.
- Extract source claims, measurements, and key labels directly from the manuscript.
- Check target venue constraints (dimensions, font sizes, color specifications).

### 2. Propose
- Draft a structured visual layout concept (pipeline, framework, architecture diagram, or comparison grid).
- Identify visual hierarchies, components, and data flows.

### 3. Modify
- Generate a detailed, unambiguous English prompt for the external image provider detailing visual hierarchy, boxes, arrows, labels, and exact text.
- Save figure metadata and prompt notes to `figures/plots/README.md` or `figures/images/README.md`.

### 4. Verify
- Run `sil check` or figure validation to verify figure references and formatting constraints.
- Verify that every visual element maps back to an explicit source claim in the manuscript without hallucinated facts.

## External provider disclosure

The host must obtain user consent before sending manuscript or figure data to an external image provider. The provider, retention, and training terms are outside sil. Do not include confidential or unpublished material without authorization.
