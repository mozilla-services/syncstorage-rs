---
name: sentry-health
description: Queries Sentry for syncstorage-rs (project syncstorage-prod) to surface error trends, new issues, regressions, and production health signals. Reports as engineer/manager view of what requires attention.
user-invocable: true
argument-hint: "new | regressions | volume | full (default: full)"
---

You are acting as **both engineer and engineering manager** for syncstorage-rs. You are reviewing Sentry to understand production error health, spot regressions, and flag issues that need triage or fixes.

Start by reading `.claude/mcp-context.md` for project slugs, environments, what to flag, and what is expected noise.

Parse `$ARGUMENTS` (trimmed, lowercased). Default is `full`. Valid modes:
- `new` — issues first seen in the last 7 days
- `regressions` — previously resolved issues that have recurred
- `volume` — high-event-count issues
- `full` — all of the above

---

## Step 1: Confirm the project

Call `mcp__sentry__find_projects` and locate the `syncstorage-prod` project under the `mozilla` organization. Confirm the project slug and organization slug before querying. If the project is not found, report the available projects and stop.

---

## Step 2: Gather issue data

Use available Sentry MCP tools to fetch issues for `syncstorage-prod`. Apply environment filter `production` unless the user has specified otherwise.

Fetch based on the selected mode:

### New issues (`new`, `full`)
Issues with `firstSeen` in the last 7 days that have not been resolved. These are the riskiest — they represent new failure modes introduced recently.

Sort by: event count descending.

For each, capture:
- Issue ID and title
- First seen timestamp
- Event count in the period
- Affected users (if available)
- Platform / culprit (the Rust file/function if available)

### Regressions (`regressions`, `full`)
Issues that were previously marked resolved but have reoccurred. These indicate incomplete fixes or flaky patches.

For each, capture:
- Issue ID and title
- Original resolution date
- Recurrence date
- Event count since recurrence

### High-volume issues (`volume`, `full`)
Issues with the most events in the last 24 hours, regardless of age. These are the most actively impacting users.

For each, capture:
- Issue ID and title
- Event count (24h)
- Is this a known/triaged issue or unacknowledged?

---

## Step 3: Classify each issue

For every issue surfaced, classify it against the known patterns in `.claude/mcp-context.md`:

**Tokenserver auth path** — involves `hawk_identifier`, FxA OAuth, `tokenserver-auth/`, `client_state`, `keys_changed_at`. These are user-facing auth failures — high priority.

**Database layer** — involves `DbError`, `diesel`, `deadpool`, connection pool, migration, query timeout. May indicate infrastructure problems.

**Syncstorage storage path** — involves BSO read/write, collection operations, quota. Usually lower priority unless volume is very high.

**Expected noise** — 409 Conflict, 503 during deployment, staging-environment-only errors. Mark as "noise" and exclude from the action list.

**Spanner-specific** — RST_STREAM, gRPC errors in production (not emulator). These should NOT appear in production — flag immediately.

---

## Step 4: Cross-reference with Jira (if context available)

If recent Jira data is available in this conversation, check whether any Sentry issues correspond to open `STOR-` bugs. Note the link if found.

If a high-volume or regression Sentry issue has no corresponding Jira ticket, that is a gap — flag it as "needs ticket".

---

## Step 5: Output format

### Production Health: [OK / Watch / Degraded]
One-line overall assessment.

### New Issues (Last 7 Days)
Table: Issue ID | Title | First Seen | Events | Classification
If none: "No new issues in the last 7 days."

### Regressions
Table: Issue ID | Title | Resolved On | Recurred On | Events Since
If none: "No regressions detected."

### High-Volume Issues (Last 24h)
Table: Issue ID | Title | Events (24h) | Triaged? | Classification
Exclude known noise (mark separately if many).

### Error Pattern Analysis
Bulleted findings only if genuine patterns are present:
- Auth path errors (tokenserver risk)
- DB layer errors (infrastructure risk)
- Sudden volume spikes (availability risk)
- Patterns that correlate with a recent deploy or migration

### Recommended Actions
Up to 5 concrete items. Each: issue ID, what to do, urgency. Skip if no action is warranted.

### Noise / Expected Errors (Acknowledged)
Brief list of issues seen but classified as expected noise. One line each. This confirms you looked at them.

---

## Guidelines

- Do not fabricate event counts or timestamps — use exactly what Sentry returns
- If a query fails or a tool is unavailable, note it and work with available data
- `is_sentry_event()` in `syncserver/src/error.rs` is the canonical routing decision — errors that are HTTP 4xx should generally not be in Sentry at high volume; flag if they are
- Focus on production environment; staging errors are informational only
- An issue being "old" does not mean it's not important — check event count trends, not just age
- Sentry issue IDs should be referenced in full (e.g. `SYNCSTORAGE-PROD-####`) if available
