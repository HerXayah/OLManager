# Governance

OLManager uses file-first governance: contributor rules, labels, branch strategy, and release expectations live in the repository, while GitHub admin settings are configured manually by maintainers.

## Branch strategy

- `main` is stable and release-oriented. It should only receive release PRs from `development` or maintainer-owned hotfixes.
- `development` is the integration branch and should become the default branch for community PRs.
- Feature, fix, docs, chore, and refactor branches are created from `development` using `type/lowercase-slug`.

Examples:

- `feat/scouting-reports`
- `fix/match-clock-stall`
- `docs/release-process`
- `chore/update-ci`

## Issue and PR gates

1. Contributor opens a templated issue.
2. Maintainer reviews scope and applies `status:approved` when the work is accepted.
3. Contributor opens a branch from `development`.
4. PR targets `development`, links the approved issue, carries exactly one `type:*` label, and passes checks.
5. Maintainers review and merge.

Release and hotfix PRs targeting `main` are maintainer-owned exceptions and must follow [`RELEASE_PROCESS.md`](RELEASE_PROCESS.md).

## Labels

Status labels:

- `status:needs-review` — default state for new issues that need maintainer triage.
- `status:approved` — maintainer-approved work; PRs may be opened against this issue.

Type labels:

- `type:bug`
- `type:feature`
- `type:docs`
- `type:chore`
- `type:refactor`
- `type:test`
- `type:release`

Each PR must have exactly one `type:*` label. Multiple type labels make release notes and triage noisy, so maintainers should ask contributors to fix labels before review.

## Required checks

Protected branches should require these checks:

- `frontend-install`
- `rust-check`

PR CI is intentionally pragmatic while pre-existing runtime test/typecheck debt is resolved:

- `frontend-install` verifies the frontend dependency graph with `npm ci`.
- `rust-check` verifies Rust formatting and workspace compilation with `cargo fmt --check` and `cargo check --workspace`.

Manual `workflow_dispatch` jobs named `frontend-full-experimental` and `rust-full-experimental` run the broader suites (`npm test`, `npm run build:types`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`). They are not default protected-branch requirements until their existing failures are fixed in focused follow-up work.

PR CI must not run Tauri production bundle builds or upload production artifacts.

## Repository settings checklist

These settings require maintainer/admin rights and are not implemented by files alone:

- Protect `main` first.
- Create `development` from the current stable branch.
- Protect `development`.
- Set the default branch to `development` after it exists.
- Require PR reviews before merging to `main` and `development`.
- Require checks `frontend-install` and `rust-check` before merging.
- Block direct pushes to protected branches.
- Disable blank issues and rely on issue forms.
- Configure labels listed above.
- Enable Discussions if the maintainers want a dedicated place for questions.
- Complete or explicitly disclose the inherited documentation audit in [`INHERITED_DOCS_AUDIT.md`](INHERITED_DOCS_AUDIT.md) before public OSS announcement.

Suggested `gh` commands after confirming repository ownership and branch names:

```bash
gh label create status:needs-review --color D4C5F9 --description "Needs maintainer triage"
gh label create status:approved --color 0E8A16 --description "Approved for implementation"
gh label create type:bug --color D73A4A --description "Bug fix"
gh label create type:feature --color A2EEEF --description "Feature work"
gh label create type:docs --color 0075CA --description "Documentation"
gh label create type:chore --color C5DEF5 --description "Maintenance"
gh label create type:refactor --color FBCA04 --description "Refactoring"
gh label create type:test --color 5319E7 --description "Testing"
gh label create type:release --color 0052CC --description "Release management"
```

## Release promotion

Normal releases flow from `development` to `main` via a release PR. The release PR verifies versions, changelog entries, release notes, provenance changes, inherited documentation audit status, and unsigned/signed artifact status. After merge, maintainers tag the release or run the release workflow.

## Hotfixes

Urgent fixes may branch from `main` and target `main`, but they must be back-merged into `development` immediately after release so community integration does not drift.
