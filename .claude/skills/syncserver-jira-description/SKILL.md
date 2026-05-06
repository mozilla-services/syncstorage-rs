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

**Background:**
Why this is needed and what it enables. 2–4 sentences. Note which service is affected — Syncstorage (storage node), Tokenserver (node allocation + FxA auth), or both.

**Acceptance Criteria:**
Observable, testable outcomes. Each item verifiable without reading the code. Include criteria for tests, metrics emission, and Sentry error classification where applicable.

**Implementation Steps:**
Numbered steps with crate paths, file paths, trait/function names, and structural guidance. Reference the nearest existing pattern for each step. For multi-backend changes, call out which backends need matching implementations. No code snippets — file locations, types, and patterns only.

**Database Migrations:** *(omit if no schema changes)*
Which migration crates need new files (`syncstorage-mysql/migrations/`, `tokenserver-postgres/migrations/`, etc.). Reminder: never edit published migrations — add new `up.sql`/`down.sql` pairs only.

**Tests:**
What needs to be tested. Unit (`#[cfg(test)]` in the relevant crate), integration (`tools/integration_tests/` pytest — requires running server), and E2E (`tokenserver/test_e2e.py` — hits FxA staging, session-scoped) coverage expectations. Reference the nearest existing test file as a pattern.

**Metrics & Error Handling:** *(omit if not applicable)*
Any Glean metrics, StatsD counters, or Sentry event classifications that should be emitted or updated. Note whether new error variants should be routed to Sentry or suppressed as metrics (see `is_sentry_event()` pattern in `syncstorage-spanner/src/error.rs`).

**Key Reference Files:**
Specific files the implementer should read before starting. One line each.

**Out of Scope:** *(omit if not needed)*

**Open Questions:** *(omit if none)*

## Guidelines

- Output the description only — no source file changes
- Implementation Steps should give enough detail to start work without follow-up questions — crate paths, file paths, and patterns, not prose
- Multi-backend tasks: always explicitly name which backends (MySQL / PostgreSQL / Spanner) need matching changes; do not say "all backends" without listing them
- Migration tasks: always reference the no-edit-published-migrations rule
- Omit redundant or obvious acceptance criteria
- Include Database Migrations, Metrics & Error Handling only when relevant
- If motivation or scope remain unclear after asking, flag as an Open Question rather than assuming
- Branch naming reminder for the assignee: Mozilla engineers use `type/description-STOR-####`; community contributors use `type/description-####`
