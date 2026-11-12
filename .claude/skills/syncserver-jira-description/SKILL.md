---
name: syncserver-jira-description
description: Drafts a concise Jira description for a syncstorage-rs task. Gathers context via targeted interview, researches relevant patterns in the repo, then outputs a clean description ready for an engineer to hand to Claude for implementation.
user-invocable: true
---

# Syncserver Jira Description

Draft a Jira description for a syncstorage-rs task. Output the description only — do not create, edit, or suggest changes to any source files.

## Step 1: Gather context

If a planning doc, epic description, or tech spec was provided, read it first and infer what, why, crates, and constraints before asking anything.

Required information:
- **What:** What is being built or changed, in one sentence
- **Why:** Motivation — user need, requirement, bug, or tech debt
- **Crates:** Which specific crate(s) will be modified (e.g. `syncstorage-spanner`, `tokenserver-auth`, `syncserver`, `tokenserver-postgres`)
- **Constraints:** DB migration required, multi-backend impact (MySQL/Postgres/Spanner), breaking API change, feature flag, FxA contract change — or none

If all four are clear from provided context, proceed directly to Step 2. Otherwise ask for only what is missing in a single message. Also invite related PRs, tickets, existing approach notes, or architecture notes that would add useful context.

## Step 2: Research

Search only the crates identified in Step 1. Find the most relevant existing patterns: similar feature, nearby handler, equivalent DB query, related error type. Expand to the broader workspace only if nothing relevant is found there.

Identify:
- Key files an implementer will need to touch
- The closest existing reference implementation to follow
- Which database backends are affected and whether the change must be replicated across MySQL, PostgreSQL, and/or Spanner
- Whether new migrations are required (and which crates: `syncstorage-mysql`, `syncstorage-postgres`, `tokenserver-mysql`, `tokenserver-postgres`)
- Whether tests, metrics (Glean/StatsD), or Sentry error handling apply

Incorporate findings directly into the draft — do not list them separately or ask for confirmation. Surface genuine blockers as Open Questions.

## Step 3: Output

This is the one Jira description template for the repo. `/jira-status create` uses the same sections; light tickets just leave most of them empty.

Keep the draft plain ASCII with no secrets. If it links webservices-infra, global-platform-admin, cloudops-infra, Confluence, SREIN or `*.mozgcp.net`, the ticket needs the `no-sync` label at create; say so in the output.

Sections, in order:

- Category: Feature Engineering, Engineering Excellence or Operational Excellence, plus one sentence why. See the work categories in `/jira-status`.
- Background: why this is needed and what it enables. 2 to 4 sentences. Note which service is affected: Syncstorage (storage node), Tokenserver (node allocation + FxA auth), or both.
- Acceptance criteria: observable, testable outcomes, each verifiable without reading the code. Include tests, metrics emission, and Sentry classification where they apply.
- Implementation steps: crate paths, file paths, trait/function names, and the nearest existing pattern for each step. For multi-backend changes, name the backends. No code snippets.
- Database migrations (omit if no schema change): which migration crates need new files. Never edit published migrations; add new `up.sql`/`down.sql` pairs only.
- Tests: unit (`#[cfg(test)]` in the crate), integration (`tools/integration_tests/`, needs a running server), and E2E (`tokenserver/test_e2e.py`, hits FxA staging). Point at the nearest existing test file.
- Metrics and error handling (omit if not applicable): Glean metrics, StatsD counters, or Sentry routing for new error variants (`ReportableError::is_sentry_event()`).
- Key reference files: files to read before starting, one per item.
- Out of scope (omit if not needed)
- Open questions (omit if none)
- Links: related tickets, PRs, docs.

Also suggest a summary (100 chars max), issue type, priority (`P0` to `P4`), labels, and parent when known, so the draft can go straight to `/jira-status create`.

## Guidelines

- Output the description only — no source file changes
- Implementation Steps should give enough detail to start work without follow-up questions — crate paths, file paths, and patterns, not prose
- Multi-backend tasks: always explicitly name which backends (MySQL / PostgreSQL / Spanner) need matching changes; do not say "all backends" without listing them
- Migration tasks: always reference the no-edit-published-migrations rule
- Omit redundant or obvious acceptance criteria
- Include database migrations and metrics/error handling only when relevant
- If motivation or scope remain unclear after asking, flag as an Open Question rather than assuming
- Branch naming reminder for the assignee: Mozilla engineers use `type/description-STOR-####`; community contributors use `type/description-####`
