# Frequently Asked Questions

## What is Sync?

Sync is a system of both a backend and client engines that are responsible for the syncing of client data to the storage server.

## When do things sync?

Engines do a full sync on a regular period (except for Tabs, see below):

- **iOS**: Every 15 minutes
- **Fenix**: Every 4 hours (after an initial delay)
- **Desktop**:
  - Initial delay:
    - Wake from sleep: 2s after wake
    - After startup: 5 minutes
  - The period varies depending on state:
    - Idle: 1 hour
    - Active: 10 minutes
    - After recently syncing something: 1.5 minutes (if we've synced something new, we temporarily change the sync delay from 10min → 1.5min until we don't have anything left to sync)

**Tabs** were changed with MR 2022 to sync every 5s after a tab change. This is sometimes called "quick writing."

## In what order do things sync?

The order of the engines is determined per-platform.

- **Desktop**: engines, prefs, passwords, tabs, bookmarks, addons, form autofill, forms, history, extension storage
- **Mobile**: Matches desktop, ignoring engines and prefs as they aren't analogous

However, clients can override this.

## What happens when I click on "Sync Now" in Firefox?

A "Sync Now" button or link in the UI on device A only initiates syncing for that same device A – it does not force all of your connected devices to sync. This is still true when "Sync now" is a contextual option (such as the Firefox sidebar) – clicking on what appears to be "Sync now" for a connected mobile device from desktop actually only initiates syncing of that desktop.

## Does every platform have its own engines?

We use shared components as much as possible to avoid every platform having its own engines.

## Can Mozilla read/inspect/decrypt a user's synced data?

No. Client data is encrypted before it leaves the client; we cannot decrypt this outside of the client, by design. Therefore, the server is extremely limited (e.g. no searching, filtering, fancy queries, etc). If you want to do operations on the data, they have to be done on the client.

## What is the structure of synced data on the storage server?

The structure of the synced data is not the same as the structure of the data in the client. You don't sync entire databases. Synced data is a set of JSON key-value stores with no relationships between them.

- In these key-value stores, the key is a GUID and the value is an encrypted blob of JSON. Each of these stores represents a "collection" (bookmarks, tabs, logins, etc).
- The JSON blobs are associated with hashed user and key IDs derived from the user's FxA. There are no identifiable relationships of this data apart from the `collection_id` foreign key that identifies what type of data it is.
- In most cases, each item in a store is its own record (e.g. each bookmark is one record). Tabs is different, as each device's tabs is in a single record.
- Because the JSON blob is encrypted, the server cannot see the content of any synced data. This means:
  - No relationships are enforced between collections
  - No atomic updates are possible across collections, only within a collection
  - Modeling a relational DB in synced data isn't viable

## Where are the user's Sync settings stored?

The storage server does not store the user's Sync settings, and neither does FxA. When a user sets which types of data to sync, those are sent to the storage server as collections that the indicated user will sync. Subsequent syncs will request to this data as part of the sync process.

## Syncing, step by step

From the client/user perspective:

1. Fetch kSync encryption key and kXCS node assignment token from FxA
2. Obtain token and storage node endpoint from token server
3. Fetch (unencrypted) associated collections (`info/collections`) and their last-modified time
4. Fetch (unencrypted) global metadata (`meta/global`) about syncing – IDs, version numbers, etc
5. Compare global sync IDs and storage versions to determine if we can sync – or if we need to start over
6. Fetch encryption and signing keys (`crypto/keys`) for future syncing
7. Sync the clients engine to download new client records and commands
8. Process incoming client commands
9. Determine and update enabled engines
10. For each enabled engine:
    - Fetch new records from the server
    - Decrypt each record and handle decryption errors
    - Resolve merge conflicts between incoming and changed records from step 10
    - Insert and update new records into the local store
    - Ask the tracker for a list of IDs changed since the last sync, and upload these records to the server
    - If an engine tracks a "backlog", make some progress on that backlog. This is only true for the history engine on desktop
11. Sync the clients engine to upload outgoing commands
12. Update `meta/global` on the server
13. Run validation for synced engines
14. Schedule next sync
