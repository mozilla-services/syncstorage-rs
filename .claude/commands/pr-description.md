Generate a PR title and summary from the current branch diff, following the repo's commit and PR conventions.

You are a PR description writer for syncstorage-rs. Your job is to inspect the current branch changes and produce a ready-to-use PR title and body that follows the conventions in CONTRIBUTING.md.

## Step 1 — Gather branch context

```bash
git diff origin/master...HEAD --name-only
git log origin/master..HEAD --oneline
git diff origin/master...HEAD --stat
```

## Step 2 — Read the full diff for context

```bash
git diff origin/master...HEAD
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

Follow the repo's `PULL_REQUEST_TEMPLATE.md` sections. Keep it short and plain.

```markdown
## Description

<A few sentences: the motivating problem, then what changed. Skip restating the title.>

## Testing

<How reviewers can verify, pre-filled from what changed (see below).>

## Issue(s)

Closes STOR-#### (or Closes #N for community contributors)
```

Pre-fill Testing based on file types changed:

- Rust source changed -> `- [ ] \`make clippy_<backend>\` passes`, `- [ ] Unit tests pass (\`make test\`)`
- Python source changed -> `- [ ] \`make ruff-lint\` and \`make mypy\` pass`, `- [ ] Integration tests pass (\`make run_local_e2e_tests\`)`
- SQL migrations added -> `- [ ] \`/migration-review\` passes`, `- [ ] Migration applies cleanly on a fresh DB`
- `.github/` changed -> `- [ ] CI workflow runs successfully on this branch`
- `CLAUDE.md` or docs changed -> `- [ ] Renders correctly on GitHub`

## Step 7 — Check the draft

Before printing it, check the draft yourself:

- Title is `type(scope): subject`, 72 chars max, lowercase after the colon, no trailing period
- Description, Testing and Issue(s) sections are all there
- Plain ASCII only, and no secrets (DSNs, tokens, keys, URLs with passwords)
- This repo is public. If the PR touches infrastructure or links webservices-infra, global-platform-admin, cloudops-infra, Confluence, SREIN or `*.mozgcp.net`, keep that detail out of the PR body, link the STOR ticket instead, and make sure that ticket was created with `no-sync`.

## Step 8 — Output

Print the final PR title and body, ready to paste into GitHub. All commits must be signed (the template says so). When opening it with `gh pr create`, add `--assignee @me`. Also include the suggested branch name if the current branch name doesn't follow `type/description-STOR-####` or `type/description-####` format.

Note any breaking changes that should be called out explicitly.
