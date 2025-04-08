# Tokenserver

## What is Tokenserver?
Tokenserver is responsible for allocating Firefox Sync users to Sync Storage nodes hosted in our Spanner GCP Backend.
Tokenserver provides the "glue" between [Firefox Accounts](https://github.com/mozilla/fxa/) and the
[SyncStorage API](https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html).

Broadly, Tokenserver is responsible for:

* Checking the user's credentials as provided by FxA.
* Sharding users across storage nodes in a way that evenly distributes server load.
* Re-assigning the user to a new storage node if their FxA encryption key changes.
* Cleaning up old data from deleted accounts.

The service was originally conceived to be a general-purpose mechanism for connecting users
to multiple different Mozilla-run services, and you can see some of the historical context
for that original design [here](https://wiki.mozilla.org/Services/Sagrada/TokenServer)
and [here](https://mozilla-services.readthedocs.io/en/latest/token/index.html).

In practice today, it is only used for connecting to Sync.

## Tokenserver API


## How Tokenserver Handles Failure Cases

### Token Expiry
When a Tokenserver token expires, Sync Storage returns a 401 code, requiring clients to get a new token. Then, clients would use their FxA OAuth Access tokens to generate a new token, if the FxA Access Token is itself expired, then Tokenserver returns a 401 itself.

### User revoking access token
The user could revoke the access token by signing out using the Mozilla Account’s Manage Account settings. In that case, clients continue to sync up to the expiry time, which is one hour. To mitigate against this case, Firefox clients currently receive push notifications from FxA instructing them to disconnect. Additionally, any requests done against FxA itself (for example to get the user’s profile data, connected devices, etc) will also trigger the client to disconnect.

### User Forgetting Their Password
When a user forgets and resets their password, their Sync keys change. The Tokenserver request includes the key ID (which is a hash of the sync key). Thus, on the next sync, Tokenserver recognizes that the password changed, and ensures that the tokens it issues point users to a new location on Sync Storage. In practice, it does that by including the Key ID itself in the Tokenserver token, which is then sent to Sync Storage.

### User Changes Their Password
This is similar to the case where users revoke their access tokens. Any devices with a not-expired access token will continue to sync until expiry, but clients will likely disconnect those clients faster than the 1 hour - however, a malicious user might be able to sync upwards of 1 hour.

## Utilities
Tokenserver has two regular running utility scripts:
1 - [Process Account Events](../tools/process_account_events.md)
2 - [Purge Old Records](../tools/purge_old_records_tokenserver.md)

For context on these processes, their purpose, and how to run them, please review their documentation pages.