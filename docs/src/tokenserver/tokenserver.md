# Tokenserver

## What is Tokenserver?
Tokenserver is responsible for allocating Firefox Sync users to Sync Storage nodes hosted in our Spanner GCP or Postgres DB Backend.
Tokenserver provides the "glue" between [Firefox Accounts](https://github.com/mozilla/fxa/) and the
[SyncStorage API](api/index.md).

Tokenserver consists of a single REST GET endpoint: `GET /1.0/<app_name>/<app_version>`, where `GET /1.0/sync/1.5` is the only endpoint used.

Broadly, Tokenserver is responsible for:

* Checking the user's credentials as provided by FxA.
* Sharding users across storage nodes in a way that evenly distributes server load.
* Re-assigning the user to a new storage node if their FxA encryption key changes.
* Cleaning up old data from deleted accounts.

In practice today, it is only used for connecting to Sync.However, the service was originally conceived to be a general-purpose mechanism for connecting users
to multiple different Mozilla-run services, and you can see some of the historical context
for that original design [here](https://wiki.mozilla.org/Services/Sagrada/TokenServer)
and [here](https://mozilla-services.readthedocs.io/en/latest/token/index.html).

## Tokenserver Crates & Their Purpose

### `tokenserver-auth`
Handles authentication logic, including:
- Token generation and validation.
- Ensuring clients are authorized before accessing Sync services.

### `tokenserver-common`
Provides shared functionality and types used across the Tokenserver ecosystem:
- Common utility functions.
- Structs and traits reused in other Tokenserver modules.

### `tokenserver-db`
Responsible for persisting and retrieving authentication/session-related data securely and efficiently.
Manages all database interactions for Tokenserver:
- Database schema definitions.
- Connection pooling and querying logic.

### `tokenserver-settings`
Handles configuration management:
- Loads and validates settings for Tokenserver.
- Supports integration with different deployment environments.

## Data Model

The core of the Tokenserver's data model is a table named `users` that maps each user to their storage
node, and that provides enough information to update that mapping over time.  Each row in the table
contains the following fields:

| Field | Description |
|-------|-------------|
| `uid` | Auto-incrementing numeric userid, created automatically for each row. |
| `service` | The service the user is accessing; in practice this is always `sync-1.5`. |
| `email` | Stable identifier for the user; in practice this is always `<fxa_uid>@api.accounts.firefox.com`. |
| `nodeid` | The storage node to which the user has been assigned. |
| `generation` | A monotonically increasing number provided by the FxA server, indicating the last time at which the user's login credentials were changed. |
| `client_state` | The hash of the user's sync encryption key. |
| `keys_changed_at` | A monotonically increasing timestamp provided by the FxA server, indicating the last time at which the user's encryption keys were changed. |
| `created_at` | Timestamp at which this node-assignment record was created. |
| `replaced_at` | Timestamp at which this node-assignment record was replaced by a newer assignment, if any. |

TThe `generation` column is used to detect when the user's FxA credentials have been changed
and to lock out clients that have not been updated with the latest credentials.
Tokenserver tracks the highest value of `generation` that it has ever seen for a user,
and rejects a number is less than that high-water mark. This was used previously with BrowserID.
However, OAuth clients do not provide a `generation` number, because OAuth tokens get revoked immediately when the user's credentials are changed.

The `client_state` column is used to detect when the user's encryption key changes.
When it sees a new value for `client_state`, Tokenserver will replace the user's node assignment
with a new one, so that data encrypted with the new key will be written into a different
storage "bucket" on the storage nodes.

The `keys_changed_at` column tracks the timestamp at which the user's encryption keys were
last changed. BrowserID clients provide this as a field in the assertion, while OAuth clients
provide it as part of the `X-KeyID` header. Tokenserver will check that changes in the value
of `keys_changed_at` always correspond to a change in `client_state`, and will use this pair of
values to construct the `fxa_kid` field that is communicated to the storage nodes.

When replacing a user's node assignment, the previous column is not deleted immediately.
Instead, it is marked as "replaced" by setting the `replaced_at` timestamp, and then a background
job periodically purges replaced rows (including making a `DELETE` request to the storage node
to clean up any old data stored under that `uid`).

For this scheme to work as intended, it's expected that storage nodes will index user data by either:

1. The tuple `(fxa_uid, fxa_kid)`, which identifies a consistent set of sync data for a particular
   user, encrypted using a particular key.
2. The numeric `uid`, which changes whenever either of the above two values change.

## How Tokenserver Handles Failure Cases

### Token Expiry
When a Tokenserver token expires, Sync Storage returns a 401 code, requiring clients to get a new token. Then, clients would use their FxA OAuth Access tokens to generate a new token, if the FxA Access Token is itself expired, then Tokenserver returns a 401 itself.

### User revoking access token
The user could revoke the access token by signing out using the Mozilla Account’s Manage Account settings. In that case, clients continue to sync up to the expiry time, which is one hour. To mitigate against this case, Firefox clients currently receive push notifications from FxA instructing them to disconnect. Additionally, any requests done against FxA itself (for example to get the user’s profile data, connected devices, etc) will also trigger the client to disconnect.

### User Changes Their Password
This is similar to the case where users revoke their access tokens. Any devices with a not-expired access token will continue to sync until expiry, but clients will likely disconnect those clients faster than the 1 hour - however, a malicious user might be able to sync upwards of 1 hour.

### User Forgetting Their Password (without a recovery key)
When a user forgets and resets their password without a recovery key, their Sync keys change. The Tokenserver request includes the key ID (which is a hash of the sync key). Thus, on the next sync, Tokenserver recognizes that the password changed, and ensures that the tokens it issues point users to a new location on Sync Storage. In practice, it does that by including the Key ID itself in the Tokenserver token, which is then sent to Sync Storage.

### User Forgetting Their Password (with a recovery key)
When a user forgets and resets their password, but has their recovery key, the behavior is similar to the password change and user revoking token cases.


## Utilities
Tokenserver has two regular running utility scripts:
1 - [Process Account Events](../tools/process_account_events.md)
2 - [Purge Old Records](../tools/purge_old_records_tokenserver.md)

For context on these processes, their purpose, and how to run them, please review their documentation pages.