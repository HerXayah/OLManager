# Skill Registry

**Delegator use only.** Any agent that launches sub-agents reads this registry to resolve compact rules, then injects them directly into sub-agent prompts. Sub-agents do NOT read this registry or individual SKILL.md files.

See `_shared/skill-resolver.md` for the full resolution protocol.

## User Skills

| Trigger | Skill | Path |
|---------|-------|------|
| When creating a pull request, opening a PR, or preparing changes for review. | branch-pr | /home/aalonso/.config/opencode/skills/branch-pr/SKILL.md |
| When creating a GitHub issue, reporting a bug, or requesting a feature. | issue-creation | /home/aalonso/.config/opencode/skills/issue-creation/SKILL.md |
| When writing Go tests, using teatest, or adding test coverage. | go-testing | /home/aalonso/.config/opencode/skills/go-testing/SKILL.md |
| When user says "judgment day", "judgment-day", "review adversarial", "dual review", "doble review", "juzgar", "que lo juzguen". | judgment-day | /home/aalonso/.config/opencode/skills/judgment-day/SKILL.md |
| When user asks to create a new skill, add agent instructions, or document patterns for AI. | skill-creator | /home/aalonso/.config/opencode/skills/skill-creator/SKILL.md |

## Compact Rules

Pre-digested rules per skill. Delegators copy matching blocks into sub-agent prompts as `## Project Standards (auto-resolved)`.

### branch-pr
- Every PR MUST link an approved issue; blank PRs without issue linkage are blocked.
- Verify the linked issue has `status:approved` before opening the PR.
- Create branches as `type/description` matching `feat|fix|chore|docs|style|refactor|perf|test|build|ci|revert` plus lowercase slug.
- Use conventional commits; never include AI attribution.
- Open PR with the repository template and add exactly one `type:*` label.
- Ensure automated checks pass before merge.

### issue-creation
- Use GitHub issue templates only; blank issues are disabled.
- Search existing issues for duplicates before creating a new one.
- Bug reports and feature requests must fill all required fields and pre-flight checkboxes.
- Newly created issues receive `status:needs-review`; maintainer approval adds `status:approved`.
- Do not open PRs until the linked issue is approved.
- Questions belong in Discussions, not issues.

### go-testing
- Prefer table-driven tests with named cases and `t.Run`.
- Test Bubbletea model state transitions directly by sending messages to `Update`.
- Use `teatest` for full TUI integration when terminal behavior matters.
- Keep assertions precise; check both returned values and error presence.
- Use golden files for stable complex output snapshots.
- Separate unit tests from integration tests when external resources or long-running setup is involved.

### judgment-day
- Before launching judges, resolve the skill registry and inject matching compact rules into every judge/fix prompt.
- Launch TWO independent blind judge sub-agents in parallel with identical target and criteria.
- Judges must not know about each other; synthesize verdicts only after both complete.
- Classify findings as confirmed, suspect from one judge, or contradiction.
- Apply fixes, then re-judge until both pass or escalate after two iterations.
- Never perform the review inline when acting as orchestrator.

### skill-creator
- Create skills only for repeated patterns, project-specific conventions, or complex workflows.
- Do not create a skill for trivial one-off guidance or when documentation already solves the problem.
- Use `skills/{skill-name}/SKILL.md` with frontmatter: name, description with Trigger, license, metadata.
- Keep skill instructions actionable: critical patterns, rules, gotchas, and examples where needed.
- Add optional assets/references only when they materially help execution.
- Validate triggers are specific enough to auto-load at the right time.

## Project Conventions

| File | Path | Notes |
|------|------|-------|
| — | — | No project-level convention files found in the project root. |

Read the convention files listed above for project-specific patterns and rules. All referenced paths have been extracted — no need to read index files to discover more.
