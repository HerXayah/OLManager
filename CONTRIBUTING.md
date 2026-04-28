# Contributing to Open League Manager

Thanks for helping OLManager become a healthy public OSS project. The workflow is intentionally strict because it keeps maintainer review predictable and protects licensing/provenance obligations.

## Issue-first workflow

- Use GitHub issue templates only. Blank issues are disabled.
- Search existing issues before opening a new one.
- Questions and open-ended support requests belong in Discussions, not issues.
- New issues start as `status:needs-review`.
- Do not start implementation or open a PR until a maintainer marks the linked issue `status:approved`.

## Branches

- `main` is stable/release-only.
- `development` is the default integration branch for community work.
- Create feature/fix/docs branches from `development`.
- Branch names SHOULD follow `type/lowercase-slug`, for example:
  - `feat/player-progression`
  - `fix/save-load-error`
  - `docs/data-provenance`
  - `chore/update-actions`

## Commits and PRs

- Use conventional commits, for example `fix: preserve save metadata`.
- Target `development` for normal contributions.
- Target `main` only for maintainer-owned release or hotfix promotion PRs.
- Link the approved issue in the PR body with `Closes #123`, `Fixes #123`, or an equivalent GitHub closing keyword.
- Apply exactly one `type:*` label to the PR.
- Confirm that affected docs and provenance notes are updated.

## Required local checks

Run relevant non-production checks before requesting review:

```bash
npm test
npm run build:types
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Do not run or require production Tauri bundle builds for PR validation.

## Provenance responsibilities

Any PR that adds or modifies inherited OpenFootManager assets, Leaguepedia-derived data, generated caches, or other third-party data must update [`docs/DATA_PROVENANCE.md`](docs/DATA_PROVENANCE.md) or an approved provenance record. If redistribution permissions are unclear, exclude the data until a maintainer approves it.

## Review expectations

Maintainers may close or convert submissions that skip issue approval, target the wrong branch, include unclear provenance, or fail checks. This is not bureaucracy for its own sake; it keeps the public project legally and operationally maintainable.
