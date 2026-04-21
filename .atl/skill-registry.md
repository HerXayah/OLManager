# Skill Registry

Generated: 2026-04-21
Project: OLManager

## Scan Sources

- User-level scanned: `~/.claude/skills/`, `~/.config/opencode/skills/`, `~/.gemini/skills/`, `~/.cursor/skills/`, `~/.copilot/skills/`
- Project-level scanned: `.claude/skills/`, `.gemini/skills/`, `.agent/skills/`, `skills/`
- Project convention files scanned: `agents.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, `GEMINI.md`, `copilot-instructions.md`

## Skills

| Skill | Source | Trigger |
|---|---|---|
| `branch-pr` | user (`~/.config/opencode/skills/branch-pr/SKILL.md`) | Creating/opening/preparing a pull request |
| `go-testing` | user (`~/.config/opencode/skills/go-testing/SKILL.md`) | Writing Go tests, Bubbletea TUI tests, or adding Go test coverage |
| `issue-creation` | user (`~/.config/opencode/skills/issue-creation/SKILL.md`) | Creating a GitHub issue, bug report, or feature request |
| `judgment-day` | user (`~/.config/opencode/skills/judgment-day/SKILL.md`) | User asks for “judgment day” / adversarial dual review |
| `skill-creator` | user (`~/.config/opencode/skills/skill-creator/SKILL.md`) | Creating new AI agent skills or documenting AI patterns |

Excluded by policy: all `sdd-*`, `_shared`, `skill-registry`.

## Project Conventions

- No project-level convention index/instruction file found in repository root (`agents.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, `GEMINI.md`, `copilot-instructions.md`).
- Effective runtime conventions are provided by global OpenCode agent instructions (`~/.config/opencode/AGENTS.md`).

## Compact Rules (auto-resolver input)

- **Go/Rust test work**: load `go-testing` for Go-specific patterns; for Rust keep native `cargo test` + integration tests under `tests/`.
- **PR workflow**: load `branch-pr` before opening or preparing PRs.
- **Issue workflow**: load `issue-creation` before creating GitHub issues.
- **Adversarial review**: load `judgment-day` when user asks for dual/blind review.
- **Skill authoring**: load `skill-creator` when requested to create or update an AI skill.
