# Release Process

OLManager releases are maintainer-owned and source-first until signing/notarization and binary packaging policy is finalized.

## Branch flow

1. Community work merges into `development`.
2. Maintainer opens a release PR from `development` to `main`.
3. Release PR verifies versions, changelog, release notes, provenance, and required checks.
4. After merge to `main`, maintainer creates a version tag or runs release dispatch.
5. Release workflow creates source archive artifacts and checksums.

## Release PR checklist

- [ ] `package.json` version is correct.
- [ ] `src-tauri/Cargo.toml` version is correct.
- [ ] `src-tauri/tauri.conf.json` version is correct.
- [ ] `CHANGELOG.md` has a dated release section.
- [ ] Release notes mention unsigned/signed artifact status.
- [ ] Data provenance changes are documented.
- [ ] [`docs/INHERITED_DOCS_AUDIT.md`](INHERITED_DOCS_AUDIT.md) is complete, or release notes explicitly disclose remaining unaudited inherited docs.
- [ ] Required PR checks `frontend-install` and `rust-check` pass.
- [ ] Manual experimental checks have been run and reviewed, or remaining failures are explicitly documented in release notes: `frontend-full-experimental` and `rust-full-experimental`.
- [ ] No production Tauri bundle build is required by PR CI.

## Version sync

The project version must stay aligned across:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

The release workflow verifies these values before producing source artifacts.

## Tags

Use semantic version tags with a `v` prefix, for example:

```text
v0.2.1
v0.3.0
```

## Artifacts

Initial releases publish source archives and SHA-256 checksums. Platform binaries, signing, notarization, and installer artifacts are intentionally postponed until maintainers configure secrets and document the support matrix.

If unsigned binaries are ever published, release notes must clearly say they are unsigned and explain the expected verification path.

## Hotfixes

Hotfixes may branch from `main` and target `main` only when the issue cannot wait for normal `development` promotion. After the hotfix release, back-merge `main` into `development` immediately.

## Signing and notarization placeholders

Potential future secrets include:

- Apple Developer ID certificate and notarization credentials.
- Windows signing certificate.
- Linux package signing key.
- GitHub release token permissions.

Do not add real secret names, credentials, or signing logic until maintainers decide the release policy.
