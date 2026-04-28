# Inherited Documentation Audit

Before OLManager is announced or released as a public open-source project, maintainers must audit documentation inherited from the original repository. Some documents may be accurate enough to keep, some may need GPL/provenance attribution updates, and some may need to be removed if they no longer describe the current project.

This checklist is intentionally repository-visible so the release decision is reviewable in PRs instead of living only in maintainer memory.

## Audit rules

For each inherited or legacy document:

- [ ] Confirm whether the document is inherited, newly authored for OLManager, or mixed.
- [ ] Confirm license/provenance status and required attribution.
- [ ] Verify technical accuracy against the current codebase.
- [ ] Choose a disposition: `keep`, `update`, `move to legacy`, or `remove`.
- [ ] Link the PR or issue that completes the disposition.

Do not publish release notes that imply these documents are fully current until this audit is complete or any known gaps are explicitly disclosed.

## Completed dispositions

The audit below covers the inherited documentation files present in the repository before the OSS governance cleanup. The decision is conservative: technical/game-system knowledge is preserved under `docs/legacy/inherited-docs/` when it may help future archaeology, while stale alpha tester material is removed because it points contributors to obsolete release channels, issue templates, product names, and log paths.

| Original document | Disposition | Rationale |
|---|---|---|
| `docs/ARCHITECTURE.md` | **Update/replace in place** | Rewritten as current contributor architecture documentation for OLManager/Tauri v2, React/TypeScript, Rust workspace crates, persistence, testing, and dependency boundaries. The older broad index links to football-era docs were removed from the public docs path. |
| `docs/DEFINITIONS.md` | **Move to legacy/reference** → `docs/legacy/inherited-docs/DEFINITIONS.md` | Contains useful historical schema notes for OpenFootManager JSON world/name/team definitions, but it is not the current public OLManager contributor guidance and must not be presented as authoritative without a fresh provenance/data audit. |
| `docs/GAME_SYSTEMS.md` | **Move to legacy/reference** → `docs/legacy/inherited-docs/GAME_SYSTEMS.md` | Preserves valuable descriptions of inherited football-era turn processing, training, staff, traits, schedules, messages, news, world generation, finance, and transfer concepts. It is stale/mixed relative to the current OLManager direction and should remain reference-only. |
| `docs/GETTING_STARTED.md` | **Move to legacy/reference** → `docs/legacy/inherited-docs/GETTING_STARTED.md` | Player-facing OpenFoot Manager guide is useful product archaeology, but it describes old football gameplay, UI labels, and user flows. Current public release docs should not advertise it as a live getting-started guide. |
| `docs/MATCH_SIMULATION.md` | **Move to legacy/reference** → `docs/legacy/inherited-docs/MATCH_SIMULATION.md` | Keeps detailed inherited match-simulation knowledge, including the 5-zone model and historical comparison to `docs/legacy/simulation.rst`. It is too specific/stale to serve as current public architecture without re-verification against active code. |
| `docs/SAVE_SYSTEM_DESIGN.md` | **Move to legacy/reference** → `docs/legacy/inherited-docs/SAVE_SYSTEM_DESIGN.md` | Retains useful save-system design history and migration notes, but public contributor guidance is now summarized in `docs/ARCHITECTURE.md`. The detailed design should be re-audited before being treated as current implementation truth. |
| `docs/performance-improvement-plan.md` | **Move to legacy/reference** → `docs/legacy/inherited-docs/performance-improvement-plan.md` | Contains useful Spanish performance findings and roadmap ideas, but it is an inherited point-in-time analysis with stale file references and should not be part of the authoritative OSS docs index. |
| `docs/ALPHA_TESTING_GUIDE.md` | **Delete** | Obsolete private alpha tester guide for Openfoot Manager 0.2.0. It references old GitHub issue templates, installer distribution assumptions, log paths, and product scope that are misleading for public OLManager OSS contributors. |
| `docs/ALPHA_TESTING_GUIDE_DE.md` | **Delete** | Translation of the obsolete alpha guide. Keeping untranslated/stale tester instructions would multiply maintenance burden and preserve wrong public-release information. |
| `docs/ALPHA_TESTING_GUIDE_ES.md` | **Delete** | Translation of the obsolete alpha guide. Public feedback now belongs in the current GitHub issue forms and governance flow, not inherited alpha-testing instructions. |
| `docs/ALPHA_TESTING_GUIDE_FR.md` | **Delete** | Translation of the obsolete alpha guide. Same stale product/distribution/template concerns as the English guide. |
| `docs/ALPHA_TESTING_GUIDE_IT.md` | **Delete** | Translation of the obsolete alpha guide. Same stale product/distribution/template concerns as the English guide. |
| `docs/ALPHA_TESTING_GUIDE_PT.md` | **Delete** | Translation of the obsolete alpha guide. Same stale product/distribution/template concerns as the English guide. |
| `docs/ALPHA_TESTING_GUIDE_PTBR.md` | **Delete** | Translation of the obsolete alpha guide. Same stale product/distribution/template concerns as the English guide. |
| `docs/legacy/simulation.rst` | **Keep as legacy/reference** | Already lives in `docs/legacy/` and clearly represents historical simulation design/provenance. Keep it out of the main docs index except through the legacy section. |

Add any additional inherited documentation, assets, screenshots, generated data, or examples discovered during future review.

## Release gate

The release PR checklist in [`RELEASE_PROCESS.md`](RELEASE_PROCESS.md) must stay blocked until one of these is true:

1. Every row above has a reviewed disposition and linked follow-up, or
2. The release notes explicitly state which inherited docs remain unaudited and why publishing is still acceptable.

For the files listed above, this audit is complete. Future release PRs still need to verify newly discovered inherited assets, screenshots, generated datasets, examples, and any legacy docs promoted back into the public/current docs set.
