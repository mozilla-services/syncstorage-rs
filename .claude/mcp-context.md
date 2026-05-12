# MCP Context: syncstorage-rs

Shared reference for Jira and Sentry MCP operations in this repository.
Skills read this file at runtime — do not expect it to be auto-loaded.

---

## Jira

- **MCP tool prefix:** `mcp__atlassian__`
- **Project key:** `STOR`
- **Board URL:** https://mozilla-hub.atlassian.net/jira/software/projects/STOR/boards (confirm with `getAccessibleAtlassianResources` if the URL changes)
- **Issue types in use:** Bug, Task, Story, Spike
- **Priority labels:** Blocker, Critical, Major, Minor, Trivial
- **Relevant components:** syncstorage, tokenserver, spanner, postgres, mysql, infrastructure
- **Typical sprint cadence:** 2-week sprints
- **Commit/branch format:** `type/description-STOR-####` (see CLAUDE.md)

### JQL patterns

```jql
-- Current sprint, all open
project = STOR AND sprint in openSprints() AND statusCategory != Done ORDER BY priority DESC

-- Current sprint, everything (for progress view)
project = STOR AND sprint in openSprints() ORDER BY statusCategory ASC, priority DESC

-- High-priority bugs, not resolved
project = STOR AND issuetype = Bug AND priority in (Blocker, Critical) AND statusCategory != Done

-- Recently resolved (last 7 days)
project = STOR AND statusCategory = Done AND resolutiondate >= -7d ORDER BY resolutiondate DESC

-- Opened in last 14 days
project = STOR AND created >= -14d ORDER BY created DESC

-- Stale in-progress (no update for 5+ days)
project = STOR AND statusCategory = "In Progress" AND updated <= -5d ORDER BY updated ASC

-- Blocking issues (linked "blocks" relationship)
project = STOR AND issue in linkedIssues("STOR-*", "blocks") AND statusCategory != Done

-- All open epics
project = STOR AND issuetype = Epic AND statusCategory != Done ORDER BY priority DESC, updated DESC

-- Children of a specific epic
project = STOR AND parent = STOR-#### ORDER BY priority DESC, status ASC

-- Children of multiple epics (batch)
project = STOR AND parent in (STOR-100, STOR-101, STOR-102)

-- Issues assigned to a specific user
project = STOR AND assignee = "user@mozilla.com" AND statusCategory != Done ORDER BY updated DESC

-- User's recently resolved work
project = STOR AND assignee = "user@mozilla.com" AND statusCategory = Done AND resolutiondate >= -30d ORDER BY resolutiondate DESC
```

### Issue link base URL

All issue links: `https://mozilla-hub.atlassian.net/browse/STOR-####`
Confirm base URL at runtime via `getAccessibleAtlassianResources`.

### Create/edit field reference

When creating or editing issues via `createJiraIssue` / `editJiraIssue`:

| Field | Value |
|---|---|
| `project` | `STOR` |
| `issuetype` | `Bug` / `Task` / `Story` / `Spike` |
| `priority` | `Blocker` / `Critical` / `Major` / `Minor` / `Trivial` |
| `summary` | ≤ 100 characters, imperative present tense |
| `description` | Structured: Background, Acceptance Criteria, Notes |
| `assignee` | Jira account ID or email (look up if needed) |
| `parent` | Epic key (e.g. `STOR-100`) if issue belongs to an epic |

Workflow transitions (via `transitionJiraIssue`): fetch available transitions first with `getTransitionsForJiraIssue` — transition names and IDs vary by project config.

### What to flag

- Any **Blocker or Critical** bug open for > 3 days without an assignee
- Issues **in progress but not updated** for 5+ days
- Sudden **spike in new bugs** vs. previous sprint baseline
- Issues labeled or component-tagged for a backend (spanner, postgres, mysql) that might indicate DB-specific regressions
- **Sprint scope creep**: new issues added mid-sprint, especially Blockers

---

## Sentry

- **MCP tool prefix:** `mcp__sentry__`
- **Organization:** mozilla (confirm with `find_projects` if slug changes)
- **Project slug:** `syncstorage-prod`
- **Environments:** production, staging
- **Primary language:** Rust (via `sentry-rust` SDK); Python errors from integration test tooling are noise

### What to flag

- **New issues** introduced in the last release or last 7 days — especially those not seen in prior releases
- **Regressions**: issues that were previously resolved and have recurred
- **High-volume errors**: issues with > 100 events in 24h that are not already triaged
- **Tokenserver auth errors**: errors from `tokenserver-auth/`, `hawk_identifier`, or FxA OAuth paths — these are user-facing auth failures
- **Database errors**: connection pool exhaustion, migration failures, query timeouts (look for `DbError`, `diesel::result::Error`, `deadpool` in fingerprints)
- **Spanner RST_STREAM / gRPC errors**: these are known-flaky from the Spanner emulator in test but should not appear in production
- **Errors that are `is_sentry_event() = false` appearing anyway**: miscategorized errors that should be suppressed

### What to treat as expected noise

- `409 Conflict` from syncstorage BSO writes (client retry behavior, not a server error)
- `503 Service Unavailable` spikes during deployments (transient)
- Test environment errors tagged with `environment: staging` during CI runs

### Error routing context

From `syncserver/src/error.rs`:
- `is_sentry_event()` controls whether errors route to Sentry vs. metrics-only
- Errors that are HTTP 4xx (client errors) are generally metrics-only
- Errors that are HTTP 5xx (server errors) route to Sentry
- `ApiError` wraps both — check the inner error type when triaging
