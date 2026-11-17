# Tokenserver Administrative API

> Unless stated otherwise, all APIs use `application/json` for the request and
> response content types.

These are backend-service endpoints called by the Firefox Accounts (FxA)
Service, not by Sync clients. No end-user credential is involved in either: both
authenticate with an FxA-signed JWT verified against the same FxA JWKs
Tokenserver already uses for OAuth.

Both are disabled by default and are not registered as routes at all unless
their setting is enabled, so a request to a disabled endpoint gets a 404 from
the router rather than a 401.

They are documented in a separate OpenAPI document from the client-facing API:

- **Swagger UI**: `https://<your-deployment-url>/admin/swagger-ui/`
- **OpenAPI spec**: `https://<your-deployment-url>/api-doc/admin-openapi.json`

See [OpenAPI Documentation](../open-api-docs.md) for how that document is
generated and previewed.

## Distinguishing the three FxA-signed credentials

All three credentials Tokenserver accepts ride the same FxA JWKS and share an
`iss`, so `iss` separates nothing on its own. `jsonwebtoken` does not match on
`kid` either — every configured JWK is tried against every verifier — so a
distinct signing key would buy nothing. Only `typ` and `aud` do real work:

| credential | `iss` | `aud` | `typ` | `exp` |
| --- | --- | --- | --- | --- |
| oldsync access token | not checked | not checked | `application/at+jwt` | required |
| webhook SET | checked if present | checked if present | none — FxA sends none | checked if present |
| delete assertion | required | required, single-valued | required, exact | required, and capped |

The delete assertion is the strict one deliberately: it authorizes an
irreversible operation, and it is the only one of the three with no legacy
traffic constraining what it may demand.

## `POST /1.0/webhooks/fxa/events`

Consumes [Security Event Tokens](https://datatracker.ietf.org/doc/html/rfc8417)
(SETs) describing account lifecycle events and applies them to the Tokenserver
`users` records.

The sender is
[fxa-event-broker](https://github.com/mozilla/fxa/tree/main/packages/fxa-event-broker),
the FxA service that fans account events out to relying parties over webhooks.
The SET envelope and the full set of event identifiers it can send are
specified in
[Account Events](https://mozilla.github.io/ecosystem-platform/reference/account-events)
— that page, not this one, is the authority on the wire format.

Set `SYNC_TOKENSERVER__FXA_WEBHOOK_ENABLED=true` to register the route.

### Authentication

```text
Authorization: Bearer <RS256 JWT>
```

The SET is the entire request; **there is no request body** — the events live
in the JWT's `events` claim.

| | |
| --- | --- |
| `iss` | per `SYNC_TOKENSERVER__FXA_WEBHOOK_SET_ISSUER` |
| `aud` | per `SYNC_TOKENSERVER__FXA_WEBHOOK_SET_CLIENT_ID`; the relying party, i.e. us |
| `sub` | the FxA uid the events concern |
| `events` | a JSON object keyed by event type URI |

Unlike the delete assertion, `exp` is not in the required claim set: FxA does
not always send one, and it is only validated when present. No `typ` is
required, because FxA sends none on SETs.

The token is tried against each configured verifier in turn — one per JWK, so
both `fxa_oauth_primary_jwk` and `fxa_oauth_secondary_jwk` are accepted while
FxA rotates keys. A signature mismatch moves on to the next key; any other
verification failure rejects immediately.

### Events

The user is identified as `<sub>@<SYNC_TOKENSERVER__FXA_EMAIL_DOMAIN>`.

- **`https://schemas.accounts.firefox.com/event/delete-user`** — retires the
  Tokenserver user record. The syncstorage data itself is *not* deleted here;
  it is left for
  [`purge_old_records.py`](../tools/purge_old_records_tokenserver.md).
- **`https://schemas.accounts.firefox.com/event/password-change`** — sets the
  user's generation to `changeTime - 1`, invalidating credentials issued before
  the change. Ignored if the event carries no `changeTime`.

Any other event type is logged and dropped. As of this writing FxA also
defines `profile-change`, `subscription-state-change`, `metrics-opt-out` and
`metrics-opt-in` under the same URI prefix; none of them affect node
assignment or credential validity, so Tokenserver ignores them. An `events`
claim that is not a JSON object is accepted and ignored.

When `SYNC_TOKENSERVER__FXA_WEBHOOK_METRICS_ONLY=true`, events are counted but
not applied — useful for measuring traffic before enabling processing.

Two statsd counters are emitted: `webhook.events_received` (incremented by the
number of events in the SET, including ones that will be dropped) and
`webhook.events_processed` (per event actually applied, tagged
`event_type:delete_user` or `event_type:password_change`).

### Responses

- **200** — events accepted, including when every event was dropped as
  unhandled and when running in metrics-only mode. The body is empty.
- **401** — the SET was missing, malformed, or not signed by any configured
  key.
- **500** — an internal failure, typically a database error while applying an
  event.

Note that a 200 means the SET was accepted, not that any particular event
changed anything.

## `DELETE /1.0/users/{fxa_uid}/storage`

Deletes a user's syncstorage data synchronously, on behalf of the FxA Auth
Server. The Tokenserver `users` record is left untouched: generation is not
bumped, nothing is marked replaced, and the next token request returns the same
`uid` and node. The client simply finds empty storage.

This is therefore a *reset sync data* operation, deliberately distinct from
stopping synchronization — connected devices keep working tokens and may
re-upload. It is also not account deletion; that remains the `delete-user`
Security Event Token on `POST /1.0/webhooks/fxa/events` above, which retires
the user record and leaves the data for
[`purge_old_records.py`](../tools/purge_old_records_tokenserver.md).

Set `SYNC_TOKENSERVER__FXA_DELETE_DATA_ENABLED=true` to register the route.

### Authentication

Callers present an FxA-signed assertion:

```text
Authorization: Bearer <RS256 JWT>
```

| | |
| --- | --- |
| `typ` (header) | `tokenserver-admin+jwt` — the credential *family*, shared by every tokenserver-admin assertion |
| `scope` | `sync_data:delete` — the *operation* |
| `iss` | the FxA OIDC issuer, per `SYNC_TOKENSERVER__FXA_ADMIN_ISSUER` |
| `aud` | per `SYNC_TOKENSERVER__FXA_ADMIN_AUDIENCE`; single-valued |
| `sub` | the FxA uid whose data to delete |
| `exp` | required, and capped server-side at ~2 minutes ahead |
| `jti` | required |

`typ` and `scope` do different jobs, and both are checked. `typ` separates
these assertions from the other FxA-signed credentials tokenserver accepts —
oldsync access tokens (`at+JWT`) and event-broker SETs (no `typ` at all).
`scope` separates tokenserver-admin operations from each other, so an
assertion minted for a different admin endpoint cannot authorize a deletion.

"Admin" names a **credential family, not a privilege level.** The
account-events webhook (`POST /1.0/webhooks/fxa/events`) is administrative too
— it retires accounts — but it is deliberately *not* part of this family. It
accepts event-broker Security Event Tokens, which state that something
happened, rather than auth-server assertions, which command an action now. A
`tokenserver-admin+jwt` is never valid there, and a SET is never valid here.

Adding a new admin operation may entail adding a `scope` to this family. It
does not mean extending the webhook, and the webhook should not grow a second
credential type.

Four details that are easy to get wrong:

- **`sub` is authoritative, not the path.** The server acts on `sub` and
  rejects the request if the uid in the path differs. The path uid is for
  routing and logs.
- **`aud` must be single-valued.** A JSON array is rejected, because audience
  matching is by intersection and an array could satisfy more than one of
  Tokenserver's verifiers at once. For the same reason, startup refuses if
  `fxa_admin_audience` equals `fxa_webhook_set_client_id` or
  `fxa_oauth_server_url`.
- **`scope` must contain the operation.** A well-formed assertion for another
  tokenserver-admin operation will be rejected here, by design.
- **`typ` must be exactly this value.** It is what keeps these assertions from
  being interchangeable with oldsync access tokens (`at+JWT`) and webhook SETs
  (which carry no `typ`). The server applies RFC 7515 §4.1.9's implicit
  `application/` prefix before comparing, so the unprefixed form above is
  correct on the wire.

`jti` is required and logged for correlation but **not** stored, so it is not
replay protection; `exp` and the server-side lifetime cap are what bound a
leaked assertion. The cap is 120 seconds plus a deliberately tight 15-second
clock-skew leeway. Within that window a replay is a no-op, because deleting
already-deleted data succeeds.

Key rotation needs no coordination: configure the incoming key as the primary
or secondary JWK and both are accepted while FxA switches over.

`tools/fxa_delete/make_delete_assertion.py` is the reference implementation and
can emit a ready-to-run `curl` with `--curl`.

### Responses

- **200** — deleted, or the user had no node assignment and so had no data.
  `deleted` reports which:

  ```json
  { "deleted": true }
  ```

  A node answering 404 also counts as `"deleted": true`: it no longer knows
  about this user. `"deleted": false` means no node was assigned, so there was
  nothing to ask — still a success.

- **401** — the assertion was missing, malformed, expired, dated too far ahead,
  signed with an unconfigured key, wrongly typed, carried the wrong `aud`/`iss`,
  lacked a `jti`, or its `sub` did not match the path uid.
- **404** — no Tokenserver record exists for this `fxa_uid`, or the record is
  retired (a deleted account). Both report a `description` of "Unknown user".
- **500** — an internal failure, including the storage node rejecting the HAWK
  credential Tokenserver minted for it, which indicates a bug on our side.
- **503** — the storage node could not be reached, timed out, or returned an
  error. Retrying is safe; deletion is idempotent.

The whole delete is bounded at 60 seconds end to end, independent of the
per-request timeout in `SYNC_TOKENSERVER__FXA_DELETE_DATA_REQUEST_TIMEOUT`, so
a slow node cannot pin a worker indefinitely.

### Scope and guarantees

Only the user's **current** record is acted on — the one Tokenserver would hand
out a token for, i.e. the greatest `(generation, created_at)`. A row marked
`replaced_at` still counts as current if it wins that ordering: it holds data
under the most recent key we know of, which is live data the user is asking us
to delete.

This matches what deletion has always meant here: a user-initiated
`DELETE /1.5/{uid}` is signed with the current `fxa_kid` too. Data filed under
a superseded `fxa_kid` is encrypted with a sync key the user no longer holds,
and is cleaned up by `purge_old_records.py`.

"Synchronous" applies to the BSO rows, which is the user-visible data. Two kinds
of cleanup remain eventual, so a 200 does not mean every byte is gone:

- On MySQL, `batch_uploads` and `batch_upload_items` have no foreign keys and
  are left to the batch TTL sweep. Postgres cascades from `user_collections`
  via foreign keys, and Spanner cascades via `INTERLEAVE IN PARENT`.
- On Spanner, GCS-offloaded payloads are reclaimed asynchronously by
  [`tools/payload-reconciler/reconcile_payload_links.py`](../tools/payload_link_reconciler.md).
