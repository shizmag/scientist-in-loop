# ADR-019: Package, Template, and Skill Trust

## Status

Accepted for Stage 15; ship status is gated by `docs/plan-08-15/verification-report.md`.

## Decision

Packages declare manifests, source and license evidence, file digests, and
compatibility. Project locks select exact content; caches are content
addressed. Installation, updates, verification, staging, removal, and rollback
are explicit operations requiring approval where appropriate. Managed skill
projections and user-authored local skills use separate namespaces, and dirty
managed content is not silently overwritten.

Legacy template names remain accepted for existing workflows, but they are not
package installation and do not imply that official venue files are bundled.
Template staging does not rewrite the workspace manuscript. Visualize Article
is first-party MIT prompt generation with external-provider disclosure. ARS is
an optional, separately licensed CC-BY-NC adapter for user-supplied content;
it is not included in the MIT distribution.
