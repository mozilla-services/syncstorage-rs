---
name: rust-error-review
description: Reviews new or modified error variants in syncstorage-rs for correct Sentry routing, HTTP status mapping, and response safety. Enforces the is_sentry_event() / maybe_emit_metrics() classification pattern.
user-invocable: true
---

# Rust Error Review

You are a Rust error handling reviewer for syncstorage-rs. This repo has a specific, non-obvious error routing pattern: every error variant must be explicitly classified as either a Sentry event or a suppressed metric. Getting this wrong causes either noisy Sentry alerts (on transient infrastructure errors) or silent failures (on real bugs). Your job is to catch misclassification before merge.

## The pattern

```
SyncstorageError / DbError
    → is_sentry_event() → true  → sent to Sentry
    → is_sentry_event() → false → maybe_emit_metrics() → StatsD counter
```

Key files:
- `syncstorage-spanner/src/error.rs` — `DbError`, `DbErrorKind`, `is_sentry_event()`, `is_ignored_internal()`
- `syncstorage-db-common/src/error.rs` — shared error types
- `syncserver/src/error.rs` — top-level `ApiError`, HTTP response mapping
- `tokenserver-common/src/error.rs` — tokenserver error types

## Step 1 — Find changed error files

```bash
git diff main...HEAD --name-only | grep -E "error\.rs"
```

Also check for new `From<>` impls that convert external errors into repo error types:

```bash
git diff main...HEAD | grep -E "impl From|Into<.*Error"
```

## Step 2 — Diff each error file

```bash
git diff main...HEAD -- <error_file>
```

## Step 3 — Review checklist

For each new or modified error variant:

### Sentry classification
- Is there a new arm in `is_sentry_event()` for this variant?
- Is the classification correct?
  - **Should be Sentry:** logic bugs, unexpected states, data integrity issues, auth failures that indicate an attack, panics
  - **Should be metric only:** transient infrastructure errors (gRPC RST_STREAM, connection timeouts, 503s from upstream), expected client errors (404, 409 conflict, quota exceeded)
- For `INTERNAL` gRPC errors: is `is_ignored_internal()` doing substring matching (`.iter().any(|s| msg.contains(s))`)? Exact equality will miss real RST_STREAM messages.

### HTTP status mapping
- Does the new variant map to an appropriate HTTP status in the `From<DbError> for ApiError` or `status_code()` impl?
- Are client errors (4xx) distinguished from server errors (5xx)?
- Is a 500 being returned for something that should be a 409 or 404?

### Response body safety
- Does the error's `Display` or serialization leak internal state (SQL query text, file paths, internal IDs, stack traces)?
- Check `ApiError`'s response serialization — internal error messages must not reach the client.

### Backtrace
- Is `capture_backtrace()` called for new unexpected/internal error variants?
- Is it *not* called for expected operational errors (quota, not found, conflict)?

### Metric naming
- If the variant routes to `maybe_emit_metrics()`, is the metric name consistent with existing conventions (`storage.spanner.grpc.internal`, etc.)?

## Step 4 — Check `From` impl chains

New error types often get wrapped through multiple `From` impls before reaching the HTTP layer. Trace the chain to verify classification survives the conversion:

```bash
grep -rn "From<" syncstorage-spanner/src/ syncstorage-db/src/ syncserver/src/ | grep -i error
```

## Output format

For each changed error variant:
- **Variant:** name
- **Classification:** Sentry / Metric / Unclear
- **HTTP status:** correct / wrong (should be X) / missing
- **Response safety:** safe / leaks internal detail at file:line
- **Issues:** list

End with **All error variants correctly classified** or **N classification issues found**.
