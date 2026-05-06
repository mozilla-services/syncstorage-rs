Examine git history of files changed in the current branch to identify regressions, re-introduced bugs, or changes that conflict with past fixes.

You are a senior engineer doing a history-aware code review. Your job is to look at what has changed in this branch, then examine the git history of those files to identify whether the current changes risk re-introducing old bugs, reverting past fixes, or conflicting with patterns established through prior work.

## Step 1 — Get the current diff

```bash
git diff main...HEAD
```

## Step 2 — Get the list of changed files

```bash
git diff main...HEAD --name-only
```

## Step 3 — For each changed file, examine recent commit history

```bash
git log --oneline -20 -- <file>
```

## Step 4 — Inspect relevant commits in full

For each commit that looks relevant (bug fixes, reverts, security patches, refactors on the same lines), inspect the full diff:

```bash
git show <commit-hash>
```

## Step 5 — Look for reverted or fix commits

```bash
git log --oneline --all --grep="revert" -- <file>
git log --oneline --all --grep="fix" -- <file>
```

Use your judgment about which historical commits are worth digging into. Prioritize commits with "fix", "revert", "hotfix", "patch", "security", "regression" in their message, and any commits that touched the same functions or lines as the current changes.

## Analysis checklist

For each changed file, work through the following. Report findings with:
- **Severity:** High / Medium / Low
- **Location:** file:line
- **Issue:** what the historical context reveals
- **Relevant commit(s):** hash + message
- **Recommendation:** what to verify or change

### 1. Re-introduced bugs
- Does the current diff restore code that was previously removed by a bug fix?
- Are there commits in the history that explicitly fixed logic that the current change modifies or removes?

### 2. Reverted or rolled-back patterns
- Has this code been reverted before? If so, why — and does the current change repeat the same pattern?
- Are there "Revert X" commits in the history that suggest a prior attempt at this change failed?

### 3. Previously fixed security issues
- Does the history show security-related fixes on these lines?
- Does the current change touch or weaken those protections?
- Pay particular attention to: HAWK/JWT token handling, FxA OAuth validation, gRPC error suppression logic, DB migration constraints.

### 4. Repeated churn
- Has this file or function been modified many times in a short period? High churn is a signal of instability.
- Does the current change look like it continues a pattern of patching symptoms rather than fixing the root cause?

### 5. Conflicting intent
- Do commit messages indicate a deliberate design decision that the current change reverses without explanation?
- Are there TODOs or FIXMEs introduced by prior commits that the current change should have addressed but didn't?

### 6. Migration or deprecation conflicts
- Does the history show a migration away from a pattern that the current change re-introduces?
- Examples: callback → async, legacy auth → FxA OAuth, MySQL-only → multi-backend.

### 7. Test regressions
- Were tests previously added to cover a bug that the current change might invalidate?
- Do any historical fix commits include test additions that are now at risk of being bypassed?

## Output format

Lead with a summary table:

| Severity | File | Issue | Relevant commit |
|---|---|---|---|

Follow with per-file detail. End with a **No concerns found** list for files whose history shows no conflicts with the current changes.

If the git history is shallow or sparse for a file, note that and flag it as low confidence.
