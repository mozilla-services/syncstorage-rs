---
name: srein-request
description: Drafts an SRE intake request (Jira SREIN) for the STOR team in the format the SREIN form expects - Service, Engineering Team, Details and context, Request, Urgency - filled from the current session's task, PRs, and tickets. Shows the draft and only files it after explicit approval.
user-invocable: true
argument-hint: "<what you need SRE to do>"
---

# SREIN Request

Draft a request to SRE for something the STOR team can't do itself (missing IAM, state buckets, admin-only consoles, project-level settings). Output a draft for review. Never file it without an explicit "post this" / "file it" from the engineer.

Form: https://mozilla-hub.atlassian.net/jira/software/c/projects/SREIN/form/1344

## Step 1: Gather context

Pull from the current session first, then ask only for what's missing:

- **Requester:** the engineer making the request. Use their display name from Jira (`atlassianUserInfo`) or git `user.name`.
- **What SRE needs to do:** one concrete action (e.g. "archive these Sentry projects", "terraform state rm these resources", "enable this API in project X").
- **Why:** the STOR ticket and what led to it.
- **Links:** every related STOR ticket, epic, PR (syncstorage-rs, webservices-infra, global-platform-admin, cloudops-infra), Sentry issue, or doc from the session.
- **Exact targets:** project IDs, resource names, Sentry project slugs, terraform addresses. Never secret values.
- **Urgency:** set by the engineer. Default to "within a week" if they don't say.

## Step 2: Draft the request

Use this exact structure. It matches past STOR requests (SREIN-1886, SREIN-1769, SREIN-1633).

```
Summary: <short imperative, <= 80 chars>

* Service: syncstorage-rs (https://github.com/mozilla-services/syncstorage-rs) : <specific component, e.g. Sentry projects, cloudops-infra state, GCP project>
* Engineering Team: STOR - syncstorage-rs (<requester name>)
* Details and context: <2 to 5 sentences: what we're doing, why, what's already done, why we need SRE (missing access, etc.)>
  Tracked in <STOR ticket URL>, <STOR epic URL>. Related: <PR, Sentry, doc URLs>
* Request: <the exact action, with a list of targets. Say what NOT to do if relevant, e.g. "state removal only, no destroy" or "archive, not delete">
* Urgency: <engineer's choice; default "Within a week">
```

Guidelines:
- Plain, colloquial tone. No em-dashes, no AI-tell words, no decorative bolding.
- Bare URLs, each on its own line or inline, never in code fences.
- Be specific about scope and safety (no destroy, archive not delete, read-only, etc.).
- Never include secret values, DSNs, tokens, or passwords. Names of secrets and resources are fine.
- Issue type: Task. Priority: P1 unless the engineer says otherwise.

## Step 3: Review and file

1. Fill a `SreinRequest` JSON (`summary`, `component`, `requester`, `details`, `tracked_in`, `related`, `request`, `urgency`, `priority`) in a temp file outside the repo and render it. This enforces the field order, the 80-char summary, and the ASCII and secrets checks:

   ```bash
   poetry -C tools/devtools run python -m devtools.models render --kind srein draft.json
   ```

   Internal links are fine here; SREIN is internal. If the request is tracked in a STOR ticket that links internal repos or docs, check that ticket has `no-sync`.
2. Show the full draft (summary + rendered fields) and stop.
3. On explicit approval, either:
   - give the engineer the form URL above to paste into, or
   - create it with `mcp__atlassian__createJiraIssue`: `cloudId` from `.claude/mcp-context.md`, `projectKey: SREIN`, `issueTypeName: Task`, `contentFormat: markdown`, description = the fields above, priority via `additional_fields: {"priority": {"name": "P1"}}`.
4. Return the new SREIN issue URL, and offer to link it from the STOR ticket with a short comment.
