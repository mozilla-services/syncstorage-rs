Generate a PR title and summary from the current branch diff, following the repo's commit and PR conventions.

You are a PR description writer for syncstorage-rs. Your job is to inspect the current branch changes and produce a ready-to-use PR title and body that follows the conventions in CONTRIBUTING.md.

## Step 1 — Gather branch context

```bash
git diff main...HEAD --name-only
git log main...HEAD --oneline
git diff main...HEAD --stat
```

## Step 2 — Read the full diff for context

```bash
git diff main...HEAD
```

Focus on understanding *what changed and why*, not just listing files.

## Step 3 — Classify the change type

Based on changed files, determine the primary `type`:

- `feat` — new functionality added
- `fix` — bug fix
- `refactor` — code restructured, no behavior change
- `test` — test additions or changes only
- `chore` — build, CI, tooling, dependencies
- `docs` — documentation only
- `perf` — performance improvement
- `style` — formatting, lint fixes only

If multiple types apply, use the most significant one.

## Step 4 — Determine scope (optional)

If the change is clearly scoped to one subsystem, add it in parentheses:
- `tokenserver`, `syncstorage`, `spanner`, `mysql`, `postgres`, `auth`, `migrations`, `ci`, `tools`

Example: `refactor(tokenserver): ...`

## Step 5 — Draft the PR title

Rules from CONTRIBUTING.md:
- Format: `type(scope): subject` or `type: subject`
- Subject: imperative present tense, no capital first letter, no trailing period
- Keep it under 70 characters

## Step 6 — Draft the PR body

```markdown
## Summary

<2–4 bullet points covering what changed and why. Lead with the motivating
problem or context, then the solution. Skip obvious restatements of the title.>

## Test plan

<Bulleted checklist pre-filled based on what changed:>
```

Pre-fill the test plan based on file types changed:

- Rust source changed → `- [ ] \`make clippy_<backend>\` passes`, `- [ ] Unit tests pass (\`make test\`)`
- Python source changed → `- [ ] \`make ruff-lint\` and \`make mypy\` pass`, `- [ ] Integration tests pass (\`make run_local_e2e_tests\`)`
- SQL migrations added → `- [ ] \`/migration-review\` passes`, `- [ ] Migration applies cleanly on a fresh DB`
- `.github/` changed → `- [ ] CI workflow runs successfully on this branch`
- `CLAUDE.md` or docs changed → `- [ ] Renders correctly on GitHub`

## Step 7 — Output

Print the final PR title and body, ready to paste into GitHub. Also include the suggested branch name if the current branch name doesn't follow `type/description-STOR-####` or `type/description-####` format.

Note any breaking changes that should be called out explicitly.
