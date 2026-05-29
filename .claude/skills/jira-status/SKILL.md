---
name: jira-status
description: Queries, summarizes, creates, and edits Jira issues for syncstorage-rs (project STOR). Supports user-centric work views, epic breakdowns, structured summaries with impact/context/highlights, and ticket creation or modification from natural language.
user-invocable: true
argument-hint: "me | epic <KEY> | epics | sprint | summary <KEY-or-text> | create <description> | update <STOR-####> <changes> | status (default: status)"
---

Start by reading `.claude/mcp-context.md` for project IDs, JQL patterns, issue types, and field conventions.

Then call `mcp__atlassian__getAccessibleAtlassianResources` to confirm the Jira cloud ID and base URL for `mozilla-hub.atlassian.net`. Store the base URL as `JIRA_BASE` — all issue links are formatted as `JIRA_BASE/browse/STOR-####`.

Parse `$ARGUMENTS` (trimmed). Match against the modes below. Default is `status`.

---

## Mode: `status` (default)

Overall project task landscape. Use when no argument is given.

Run these JQL queries in parallel:

```jql
-- Open sprint issues
project = STOR AND sprint in openSprints() AND statusCategory != Done ORDER BY priority DESC, updated DESC

-- Unassigned open issues
project = STOR AND statusCategory != Done AND assignee is EMPTY ORDER BY priority DESC

-- Recently updated (last 3 days)
project = STOR AND updated >= -3d AND statusCategory != Done ORDER BY updated DESC
```

**Output:**

### Project Status
One-line overall signal: `Active` / `Needs attention` / `Stalled`.

### Current Sprint
Table: Key (linked) | Summary | Status | Assignee | Priority
Group by status category (In Progress first, then To Do, then Done).

### Unassigned Open Issues
Table: Key (linked) | Summary | Priority | Issue Type
Flag Blockers and Criticals explicitly.

### Recently Updated
Table: Key (linked) | Summary | Updated | By

---

## Mode: `me`

Work assigned to the person running this skill. Ask: "What is your Jira email or display name?" — use their response to filter.

```jql
project = STOR AND assignee = "<user>" AND statusCategory != Done ORDER BY updated DESC
```

Also fetch recently completed by the user (for summary context):
```jql
project = STOR AND assignee = "<user>" AND statusCategory = Done AND resolutiondate >= -30d ORDER BY resolutiondate DESC
```

**Output:**

### Your Active Work
Table: Key (linked) | Summary | Status | Priority | Last Updated
Highlight anything In Progress that hasn't been updated in > 3 days.

### Your Recent Completions (Last 30 Days)
Table: Key (linked) | Summary | Resolved On | Issue Type

### Work Summary
A plain-English paragraph for each active item: what it is, where it stands, and whether anything is blocking it. Lead with the highest-priority item.

---

## Mode: `epic <KEY>`

Retrieve a specific epic and all its child issues.

Steps:
1. Call `mcp__atlassian__getJiraIssue` with the provided KEY to get the epic details.
2. Fetch all children:
   ```jql
   project = STOR AND parent = <KEY> ORDER BY priority DESC, status ASC
   ```
3. Also fetch linked issues (blocks/is blocked by) on the epic itself.

**Output:**

### Epic: [STOR-####](JIRA_BASE/browse/STOR-####) — {Epic Summary}
**Status:** {status} | **Assignee:** {assignee or Unassigned} | **Priority:** {priority}

**Description:**
{epic description, condensed to key intent — 2–4 sentences max}

### Child Issues
Table: Key (linked) | Summary | Status | Assignee | Priority | Issue Type
Group: In Progress → To Do → Done (collapsed count for Done if > 5)

### Epic Progress
- Total issues: N
- Done: N (N%)
- In Progress: N
- Blocked: N (flag if any)

### Blockers & Dependencies
List any linked "blocks" or "is blocked by" relationships. If none, omit this section.

---

## Mode: `epics`

List all open epics in the STOR project with their child counts and progress.

```jql
project = STOR AND issuetype = Epic AND statusCategory != Done ORDER BY priority DESC, updated DESC
```

For each epic, fetch child counts via:
```jql
project = STOR AND parent = <EPIC_KEY>
```
(batch these — fetch all children in one query with `parent in (KEY1, KEY2, ...)` if the tool supports it, otherwise query each epic individually)

**Output:**

### Open Epics

For each epic:
**[STOR-####](link) — {Summary}**
Status: {status} | Assignee: {assignee} | Children: {done}/{total}
One-line description of the epic's goal (from its description field, condensed).

---

## Mode: `sprint`

Current sprint state focused on progress and blockers — not health framing, just what is happening.

```jql
project = STOR AND sprint in openSprints() ORDER BY statusCategory ASC, priority DESC
```

**Output:**

### Sprint: {Sprint Name}
Dates: {start} → {end} | Days remaining: N

**In Progress** (N issues)
Table: Key (linked) | Summary | Assignee | Days In Progress

**To Do** (N issues)
Table: Key (linked) | Summary | Assignee | Priority

**Done** (N issues)
Count only unless ≤ 5 — then list.

**Blockers in sprint:**
List any issue linked as "blocks" another sprint issue, or with priority Blocker.

---

## Mode: `summary <KEY-or-scope>`

Generate a structured summary suitable for a status update, PR description, or stakeholder report. The argument can be:
- A single issue key: `summary STOR-1234`
- An epic key: `summary STOR-100` (auto-detects if it's an epic)
- A freeform scope description: `summary tokenserver auth work` (runs JQL to find matching issues)
- `summary me` — summarize the current user's recent work (ask for their name if not known)
- `summary sprint` — summarize the current sprint's work

**Steps:**
1. Resolve the scope to a set of issues. Fetch full details for each.
2. For epics: fetch children too.
3. Identify: what was done, what is in progress, what is blocked.
4. Synthesize the summary.

**Output format — always structured as:**

### Impact
What this work changes for users or the system. Lead with the most significant outcome. Use concrete language: "reduces", "enables", "removes", "fixes". If there are multiple issues, consolidate into the top 2–3 outcomes. Link relevant issues inline: e.g. "Adds PostgreSQL support for tokenserver ([STOR-42](link))."

### Context
Why this work was needed. Background the reader should have to understand the impact. Reference the driving requirement, bug, or design decision. Keep to 3–5 sentences. Link to the primary issue or epic.

### Highlights
Notable implementation decisions, technical wins, or interesting constraints resolved. Bullet points. Each item links to the relevant issue. Focus on what would surprise or interest a fellow engineer — not just a list of what was done.

---

## Mode: `create <description>`

Create a new Jira issue in STOR from natural language.

**Steps:**

1. Parse the description to infer:
   - **Summary** — one clear sentence (≤ 100 chars)
   - **Issue type** — Bug / Task / Story / Spike (infer from language; ask if ambiguous)
   - **Priority** — infer from language ("critical", "blocking" → Critical; "nice to have" → Minor; default → Major). Valid STOR priorities: `Blocker`, `Critical`, `Major`, `Minor`. `Trivial` is rejected by the project — do not use it.
   - **Work category** — `Feature Engineering` / `Engineering Excellence` / `Operational Excellence`. See [Work categories](#work-categories) below for definitions and inference rules. Always classify; if unsure between EE and OpEx, the planned/reactive distinction decides it.
   - **Assignee** — if this `create` invocation is part of the same workflow that opens a GitHub PR for the work, set the assignee to the current user. Look up their accountId via `mcp__atlassian__lookupJiraAccountId` using the email from the runtime `userEmail` context. Otherwise leave unassigned unless the user named a specific person.
   - **Parent** — if the description references an epic (e.g. "under STOR-506", "in STOR-506", "linked to <epic>"), set `parent` to that key when calling `createJiraIssue`.
   - **Description body** — expand the user's language into a structured description, leading with the category:
     - **Category:** {one of the three categories} — *one-sentence rationale*
     - **Background:** why this is needed
     - **Acceptance Criteria:** observable, testable outcomes
     - **Notes:** any constraints or references mentioned

2. Show the draft to the user before creating:
   ```
   Ready to create:
   Type:     {type}
   Project:  STOR
   Parent:   {parent key or none}
   Priority: {priority}
   Category: {category}
   Assignee: {display name or "unassigned"}
   Summary:  {summary}
   Description:
   {description}

   Create this issue? (yes / edit first / cancel)
   ```

3. On confirmation, call `mcp__atlassian__createJiraIssue` with:
   - `projectKey: STOR`
   - `issueTypeName: {type}`
   - `summary: {summary}`
   - `description: {description}` with `contentFormat: markdown`
   - `additional_fields: { "priority": { "name": "{priority}" } }`
   - `assignee_account_id: {accountId}` — only when auto-assignment applies or the user named an assignee
   - `parent: {epic_key}` — only when a parent epic was identified

4. Return the created issue key and direct link: `[STOR-####](JIRA_BASE/browse/STOR-####)`

---

## Mode: `update <STOR-####> <changes>`

Modify an existing issue. Changes can be natural language.

**Steps:**

1. Fetch the current issue with `mcp__atlassian__getJiraIssue`.

2. Parse the `<changes>` to determine what to modify:
   - Field updates (summary, description, priority, assignee) → use `mcp__atlassian__editJiraIssue`
   - Status transition ("mark as done", "move to in progress") → use `mcp__atlassian__getTransitionsForJiraIssue` then `mcp__atlassian__transitionJiraIssue`
   - Comment ("add a comment: ...") → use `mcp__atlassian__addCommentToJiraIssue`

3. Show the proposed changes before applying:
   ```
   Updating STOR-####: {current summary}
   Changes:
   - {field}: "{old value}" → "{new value}"
   - {transition}: {new status}
   - Comment: "{comment text}"

   Apply? (yes / edit / cancel)
   ```

4. Apply on confirmation. Return confirmation with the issue link.

---

## General guidelines

- **All issue references must be linked**: format as `[STOR-####](JIRA_BASE/browse/STOR-####)` — never bare key text
- **Summaries lead with impact**: what changes for users or the system, not what was done mechanically
- **Do not fabricate data**: if a query returns nothing, say so plainly
- **Ask once for missing identity**: if the user's Jira name is needed and not known, ask once; do not repeat the question across modes in the same session
- **Confirm before write operations**: always show a draft before calling `createJiraIssue`, `editJiraIssue`, `transitionJiraIssue`, or `addCommentToJiraIssue`
- **Permissions**: create/edit/transition operations require the user's Jira account to have project contributor access to STOR; if a call fails with 403, report it clearly and do not retry
- **Issue type inference**: "bug" → Bug; "task"/"chore" → Task; "feature"/"story"/"as a user" → Story; "investigate"/"explore"/"spike" → Spike
- **Classify every ticket**: when creating or summarizing, identify the [work category](#work-categories) (Feature Engineering / Engineering Excellence / Operational Excellence). Include it as the leading line of the description body for created tickets.
- **Auto-assign when PR-bundled**: if a `create` invocation is part of the same workflow that opens a GitHub PR for the same work, set the assignee to the current user (look up their accountId with `mcp__atlassian__lookupJiraAccountId`). Setting an assignee may transition the issue to In Progress via Jira workflow rules — that's expected.

---

## Work categories

Per [Feature Engineering, Engineering Excellence, and Operational Excellence](https://mozilla-hub.atlassian.net/wiki/spaces/IP/pages/1397817345). Classify every STOR ticket into one of three categories. Target distribution per quarter: ≤ 50% Feature Engineering, ≤ 30% Engineering Excellence, ≤ 20% Operational Excellence.

### Feature Engineering — *build & deliver new things*

Planned work that changes what a customer (internal or external) sees beyond speed, reliability, or bugfixes.

Examples: new API endpoints, new auth methods (passkeys), launching a region or platform, UI/UX redesigns, A/B testing, expanded user-configuration options, changes to core business logic, bug fixes against unlaunched feature work.

### Engineering Excellence — *sustain & improve velocity*

Planned work that improves code maintainability, reduces tech debt, or boosts engineering velocity. Most non-hotfix bug fixes land here. "Keeping the lights on" proactively.

Examples: refactors, test-coverage improvements, observability and monitoring, CI/CD tooling, dependency upgrades, removing dead code, fixing flaky tests, reducing on-call toil, automating manual deployments, fixes for issues in already-released features.

### Operational Excellence — *react & keep production healthy*

Unplanned work. Reliability, stability, security, and operational efficiency in the production environment. "Keeping the lights on" reactively.

Examples: security patches and CVE remediations, manual scaling under load, incident response and postmortems, hotfix patches for production incidents, resolving performance issues impacting SLAs.

### Inference rules

- **Planned vs. unplanned** decides EE vs. OpEx: if it was on a roadmap/sprint plan, it's EE; if it interrupted planned work to keep prod healthy, it's OpEx.
- **Customer-visible change** decides FE vs. EE: if the customer sees something new (beyond speed/reliability/bugfix), it's FE.
- **Dependency upgrades, CI/CD tooling, refactors, test improvements** → Engineering Excellence (unless the upgrade is a CVE remediation, then OpEx).
- **Security patches and CVEs** → Operational Excellence.
- **Bug fixes**: pre-launch → FE; against released features, planned → EE; via hotfix → OpEx.
