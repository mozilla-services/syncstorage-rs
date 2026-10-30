---
name: rust-error-review
description: Reviews new or modified error variants in syncstorage-rs for correct Sentry routing, HTTP status mapping, and response safety. Enforces the is_sentry_event() / maybe_emit_metrics() classification pattern.
user-invocable: true
---

# Rust Error Review

You are a Rust error handling reviewer for syncstorage-rs. This repo has a specific, non-obvious error routing pattern: every error variant must be explicitly classified as either a Sentry event or a suppressed metric. Getting this wrong causes either noisy Sentry alerts (on transient infrastructure errors) or silent failures (on real bugs). Your job is to catch misclassification before merge.

## The pattern

Every reportable error implements the `ReportableError` trait (`syncserver-common/src/lib.rs`): `is_sentry_event()`, `metric_label()`, `backtrace()`, `tags()`, `reportable_source()`. The Sentry middleware (`syncserver-common/src/middleware/sentry.rs`) asks `is_sentry_event()`:

```
error -> is_sentry_event() true  -> event sent to Sentry
      -> is_sentry_event() false -> maybe_emit_metrics() -> StatsD counter named by metric_label()
```

`ApiError::is_sentry_event()` (`syncserver/src/error.rs`) only reports 5xx statuses, then defers to the DB error or to whether a `metric_label()` exists.

Key files:
- `syncserver-common/src/lib.rs` - the `ReportableError` trait
- `syncserver-common/src/middleware/sentry.rs` - routing, `maybe_emit_metrics()`, header scrubbing
- `syncserver/src/error.rs` - top-level `ApiError`, HTTP status mapping, the 5xx gate
- `syncstorage-spanner/src/error.rs` - `DbError`, `DbErrorKind`, `is_ignored_internal()`
- `syncstorage-db-common/src/error.rs`, `syncserver-db-common/src/error.rs` - shared DB error types
- `tokenserver-common/src/error.rs`, `tokenserver-db-common/src/error.rs` - tokenserver error types

## Step 1 -- Find changed error files

```bash
poetry -C tools/devtools run python -m devtools.changed_scope
```

The `error_files` list is the set to review (`error.rs` files and anything under an `error/` module).

Also check for new `From<>` impls that convert external errors into repo error types:

```bash
git diff origin/master...HEAD | grep -E "impl From|Into<.*Error"
```

## Step 2 -- Diff each error file

```bash
git diff origin/master...HEAD -- <error_file>
```

## Step 3 -- Review checklist

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
- Check `ApiError`'s response serialization -- internal error messages must not reach the client.

### Backtrace
- Does `backtrace()` return a captured backtrace for new unexpected/internal variants?
- Is capture skipped for expected operational errors (quota, not found, conflict)?

### Metric naming
- If the variant is metric-only, does `metric_label()` return a name, and is it consistent with existing conventions (`storage.spanner.grpc.internal`, etc.)?

## Step 4 -- Check `From` impl chains

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
