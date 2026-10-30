# devtools

Small, deterministic helpers that the repo's Claude skills (`.claude/skills`, `.claude/commands`) call instead of doing the same steps by hand each time. Each one prints JSON so a skill can read the result without guessing.

```
poetry -C tools/devtools install
poetry -C tools/devtools run python -m devtools.changed_scope
poetry -C tools/devtools run python -m devtools.migration_check
poetry -C tools/devtools run python -m devtools.draft_lint --kind pr draft.md
poetry -C tools/devtools run python -m devtools.models render --kind srein draft.json
```

| Module | What it does |
|---|---|
| `changed_scope` | Lists files changed against `origin/master` and maps them to backends, clippy targets, trait/error files, migrations, and Python changes |
| `migration_check` | Flags edits to published migrations or `schema.ddl`, missing or empty `down.sql`, out-of-order timestamps, and destructive SQL |
| `draft_lint` | Checks a Jira, PR, SREIN or comment draft for non-ASCII characters, secrets, required sections, and internal links that need `no-sync` |
| `models` | pydantic models for SREIN, Jira and PR drafts; validates JSON and renders the final text |

`changed_scope`, `migration_check` and `draft_lint` use only the standard library. `models` needs pydantic.

Tests: `poetry -C tools/devtools run pytest`
