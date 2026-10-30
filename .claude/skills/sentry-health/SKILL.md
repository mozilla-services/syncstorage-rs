---
name: sentry-health
description: Queries Sentry for syncstorage-rs (project syncstorage-rs) to surface error trends, new issues, regressions, and production health signals. Reports as engineer/manager view of what requires attention.
user-invocable: true
argument-hint: "new | regressions | volume | full | triage <issue id or URL> (default: full)"
---

You are acting as **both engineer and engineering manager** for syncstorage-rs. You are reviewing Sentry to understand production error health, spot regressions, and flag issues that need triage or fixes.

Start by reading `.claude/mcp-context.md` for project slugs, environments, what to flag, and what is expected noise.

Parse `$ARGUMENTS` (trimmed, lowercased). Default is `full`. Valid modes:
- `new` -- issues first seen in the last 7 days
- `regressions` -- previously resolved issues that have recurred
- `volume` -- high-event-count issues
- `full` -- all of the above
- `triage <issue>` -- one issue in depth: where it comes from, whether it should be in Sentry at all, and what to do with it. Skips Steps 2 to 5; go to "Mode: triage" at the end.

---

## Step 1: Confirm the project

Call `mcp__sentry__find_projects` and locate the `syncstorage-rs` project under the `mozilla` organization. Confirm the project slug and organization slug before querying. If the project is not found, report the available projects and stop.

---

## Step 2: Gather issue data

Use available Sentry MCP tools to fetch issues for `syncstorage-rs`. Apply environment filter `prod` unless the user has specified otherwise.

Fetch based on the selected mode:

### New issues (`new`, `full`)
Issues with `firstSeen` in the last 7 days that have not been resolved. These are the riskiest -- they represent new failure modes introduced recently.

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

**Tokenserver auth path** -- involves `HawkIdentifier` (`web/extractors/hawk_identifier.rs`), FxA OAuth, `tokenserver-auth/`, `client_state`, `keys_changed_at`. These are user-facing auth failures -- high priority.

**Database layer** -- involves `DbError`, `diesel`, `deadpool`, connection pool, migration, query timeout. May indicate infrastructure problems.

**Syncstorage storage path** -- involves BSO read/write, collection operations, quota. Usually lower priority unless volume is very high.

**Expected noise** -- 409 Conflict, 503 during deployment, errors only seen in `stage` or `dev`. Mark as "noise" and exclude from the action list.

**Spanner-specific** -- RST_STREAM, gRPC errors in production (not emulator). These should NOT appear in production -- flag immediately.

---

## Step 4: Cross-reference with Jira (if context available)

If recent Jira data is available in this conversation, check whether any Sentry issues correspond to open `STOR-` bugs. Note the link if found.

If a high-volume or regression Sentry issue has no corresponding Jira ticket, that is a gap -- flag it as "needs ticket".

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

- Do not fabricate event counts or timestamps -- use exactly what Sentry returns
- If a query fails or a tool is unavailable, note it and work with available data
- `is_sentry_event()` in `syncserver/src/error.rs` is the canonical routing decision -- errors that are HTTP 4xx should generally not be in Sentry at high volume; flag if they are
- Focus on `prod`; `stage` and `dev` errors are informational only
- An issue being "old" does not mean it's not important -- check event count trends, not just age
- Sentry issue IDs should be referenced in full (e.g. `SYNCSTORAGE-RS-####`) if available

---

## Mode: triage

Read-only until the engineer says otherwise. Never resolve, ignore or assign the issue without an explicit go-ahead.

1. **Fetch it.** Use `mcp__sentry__get_sentry_resource` with the issue URL or ID (`SYNCSTORAGE-RS-####`), and pull the latest event. Note the title, culprit, first/last seen, event and user counts, `environment`, and `release`.
2. **Which service.** Read the `server_name` tag: `tokenserver-*` or `debug-tokenserver-*` is tokenserver, `syncapp-*` is a storage node. Note the environment; `stage` and `dev` are informational unless the same issue is also in `prod`.
3. **Map to code.** Take the in-app frames from the stack trace and open those files in the repo. Name the error type and variant.
4. **Should it be in Sentry?** Check the variant against `ReportableError::is_sentry_event()` and the HTTP status it maps to (see `/rust-error-review`). A 4xx or an expected transient error at volume means the routing is wrong; that's a code fix, not an ignore.
5. **Recent changes.** `git log --oneline -10 origin/master -- <files from step 3>` and compare commit dates with first seen and the `release` tag.
6. **Existing ticket.** Search STOR with JQL `project = STOR AND text ~ "<short id or title words>"` to avoid filing a duplicate.
7. **Recommend one of:**
   - resolve: test or one-off event, already fixed, or noise listed in `.claude/mcp-context.md`
   - ignore until escalating: real but low value, no user impact
   - fix routing: shouldn't reach Sentry (step 4)
   - file a ticket: real bug with no existing ticket. Draft it with `/jira-status create`; add `no-sync` if the description needs infra detail or internal links.

Output:

```
SYNCSTORAGE-RS-####: <title>
Service: tokenserver | storage node   Env: <env>   Release: <release>
Events: <n> (<users> users)   First seen: <date>   Last seen: <date>
Code: <file:line> -- <error variant>
Sentry routing: correct | should be metrics-only (why)
Related: <commit or STOR ticket, or none>
Recommendation: <one of the above> -- <one sentence why>
```

Give the Sentry issue URL on its own line after the block.
