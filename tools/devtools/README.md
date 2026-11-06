# devtools

Small, deterministic helpers that the repo's Claude skills (`.claude/skills`, `.claude/commands`) call instead of doing the same steps by hand each time. Each one prints JSON so a skill can read the result without guessing.

```
poetry -C tools/devtools install
poetry -C tools/devtools run python -m devtools.changed_scope
poetry -C tools/devtools run python -m devtools.migration_check
poetry -C tools/devtools run python -m devtools.draft_lint --kind pr draft.md
poetry -C tools/devtools run python -m devtools.models render --kind srein draft.json
poetry -C tools/devtools run python -m devtools.models schema --kind jira
```

| Module | What it does |
|---|---|
| `changed_scope` | Lists files changed against `origin/master` and maps them to backends, clippy targets, trait/error files, migrations, and Python changes |
| `migration_check` | Flags edits to published migrations or `schema.ddl`, missing or empty `down.sql`, out-of-order timestamps, and destructive SQL |
| `draft_lint` | Checks a Jira, PR, SREIN or comment draft for non-ASCII characters, secrets, required sections, and internal links that need `no-sync` |
| `models` | pydantic models for SREIN, Jira and PR drafts; validates JSON, runs `draft_lint` on the result, and renders the final text. `schema` prints the JSON shape to ask an LLM for (the same models work as a pydantic-ai `output_type`) |

`changed_scope`, `migration_check` and `draft_lint` use only the standard library. `models` needs pydantic.

Tests: `poetry -C tools/devtools run pytest`

The root `make ruff-lint`, `make mypy`, `make bandit` and `make pydocstyle` cover this directory too. For the stricter mypy config here:

```
cd tools/devtools && mypy --explicit-package-bases --python-executable "$(poetry env info -e)" devtools tests
```
