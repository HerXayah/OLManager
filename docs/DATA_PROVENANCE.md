# Data Provenance

OLManager separates GPL-inherited code/assets from third-party data. This matters because an external data source such as Leaguepedia has its own terms, attribution expectations, and redistribution constraints.

## Policy

- Treat code and assets inherited from OpenFootManager as GPL-3.0-compatible unless audited otherwise.
- Do not assume Leaguepedia or other external data becomes GPL because OLManager is GPL.
- Do not commit unclear, proprietary, or non-redistributable data.
- Prefer reproducible generators or documented extraction scripts over opaque generated files.
- Keep generated/cache files out of releases unless redistribution rights are documented.

## Required provenance record

For every external data source or inherited asset, record:

- Source name.
- Source URL.
- Source terms/license URL.
- Required attribution text.
- Extraction or retrieval date.
- Transformation process or generator command.
- Whether redistribution is allowed.
- Whether the committed file is source, generated output, or cache.
- Maintainer approval link or issue number.

## Template

```markdown
## Source: <name>

- Source URL: <url>
- Terms/license URL: <url>
- Attribution: <required text>
- Retrieved/extracted on: YYYY-MM-DD
- Data type: source | generated | cache
- Transformation: <script/process>
- Redistribution status: allowed | unclear | not allowed
- Approval issue/PR: #<number>
- Notes: <additional constraints>
```

## Leaguepedia guidance

Leaguepedia-derived data requires special care:

- Link the exact pages, API endpoints, or dumps used.
- Record the date and method of extraction.
- Preserve attribution required by the source.
- Confirm whether redistribution of raw or transformed data is allowed.
- If permission is unclear, keep extraction tooling and documentation but exclude the dataset from commits/releases until maintainers approve.

## PR checklist for data changes

- [ ] Provenance record added or updated.
- [ ] Source terms reviewed.
- [ ] Attribution included where required.
- [ ] Generated/cache status documented.
- [ ] Redistribution permission is clear.
- [ ] Maintainer approval issue is linked.
