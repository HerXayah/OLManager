# Open League Manager

Open League Manager (OLManager) is a public, GPL-3.0 desktop management game built with Tauri v2, Rust, React, and TypeScript. The project continues the OpenFootManager lineage while focusing on transparent community contribution, maintainable releases, and careful data provenance.

## Project status

OLManager is pre-alpha software. Expect incomplete gameplay systems, evolving save formats, and frequent documentation updates while the project is prepared for public open-source collaboration.

## License and lineage

This repository is licensed under the GNU General Public License v3.0. See [`LICENSE`](LICENSE).

Code and assets inherited from OpenFootManager are treated as GPL-3.0-compatible unless a later audit documents otherwise. Third-party datasets, generated caches, and source-derived content such as Leaguepedia data are **not** automatically GPL by inheritance; they require separate provenance, attribution, and redistribution review. See [`docs/DATA_PROVENANCE.md`](docs/DATA_PROVENANCE.md).

## Local development checks

Install dependencies first:

```bash
npm ci
```

Run the stable non-production checks used by required PR validation:

```bash
npm ci
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
```

Broader non-production checks are still useful, but currently tracked as pre-existing runtime/test debt and exposed through manual experimental CI jobs instead of protected-branch requirements:

```bash
npm test
npm run build:types
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Do not run production Tauri bundle builds as part of normal PR validation. Packaging belongs to the release process.

## Contributing

Contributions are issue-first:

1. Open a template-based issue or join Discussions for questions.
2. Wait for maintainer approval via `status:approved`.
3. Branch from `development` using `type/lowercase-slug`, for example `fix/ci-labels`.
4. Open the PR against `development` unless it is a maintainer release or hotfix promotion.

Start with [`CONTRIBUTING.md`](CONTRIBUTING.md), then review:

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — system overview, React/Tauri boundary, Rust crates, persistence, testing, and feature-extension rules.
- [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md) — branch model, labels, review gates, and repository settings.
- [`docs/RELEASE_PROCESS.md`](docs/RELEASE_PROCESS.md) — release PRs, version sync, tags, artifacts, and unsigned status rules.
- [`docs/INHERITED_DOCS_AUDIT.md`](docs/INHERITED_DOCS_AUDIT.md) — required audit follow-up for inherited documentation before public OSS release.
- [`docs/DATA_PROVENANCE.md`](docs/DATA_PROVENANCE.md) — external data and asset provenance requirements.
- [`SECURITY.md`](SECURITY.md) — private vulnerability reporting guidance.

## Documentation

The main documentation index is [`docs/README.md`](docs/README.md).
