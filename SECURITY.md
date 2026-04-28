# Security Policy

## Supported versions

OLManager is currently pre-alpha. Security fixes are handled on the active `development` line and promoted to `main` through the release/hotfix process.

## Reporting a vulnerability

Please do **not** open a public issue for suspected vulnerabilities.

Until a dedicated security address is published, report privately to the repository owner or the maintainer contact listed in [`MAINTAINERS.md`](MAINTAINERS.md). Include:

- Affected version or commit.
- Reproduction steps.
- Expected and actual impact.
- Whether the issue involves bundled assets, generated data, or third-party source data.

Maintainers will acknowledge valid reports as soon as practical, coordinate fixes privately when appropriate, and publish disclosure notes after a fix is available.

## Secrets and signing material

Do not commit signing certificates, notarization credentials, API keys, private datasets, or generated artifacts that contain secrets. Release signing and notarization secrets are documented placeholders until maintainers explicitly configure them.
