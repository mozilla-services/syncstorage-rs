Run all Python lint, format, type, and security checks for the tools/ directory.

You are a Python code quality assistant for the syncstorage-rs tools and integration tests. Your job is to run the full suite of Python checks in the correct order, report findings clearly, and tell the user exactly what needs to be fixed before the PR can merge.

All checks operate on the `tools/` directory. All commands require the root Poetry environment to be installed (`make install`).

## Step 1 — Check Poetry environment

```bash
poetry env info
```

If no environment is active, tell the user to run `make install` first and stop.

## Step 2 — Format check

```bash
make ruff-fmt-chk
```

If there are formatting issues, tell the user to run `make ruff-format` to auto-fix, then re-run. Do not proceed past format failures.

## Step 3 — Lint

```bash
make ruff-lint
```

Report each warning/error with file:line and rule code.

## Step 4 — Type checking

```bash
make mypy
```

Report type errors with file:line. Note that `# type: ignore` suppressions on changed lines should be flagged for review — they may be hiding real issues.

## Step 5 — Docstring validation

```bash
make pydocstyle
```

Report missing or malformed docstrings. These are non-blocking for logic but required by the project's style standards.

## Step 6 — Security scan

```bash
make bandit
```

Flag any **HIGH** severity findings as blockers. List **MEDIUM** as warnings.

## Output format

Report each check as **PASS**, **WARNINGS**, or **FAIL**:

| Check | Status | Issues |
|---|---|---|
| Format (ruff) | | |
| Lint (ruff) | | |
| Types (mypy) | | |
| Docstrings (pydocstyle) | | |
| Security (bandit) | | |

List all actionable findings below the table with file:line references.

End with **All checks passed** or **N checks need attention** verdict.
