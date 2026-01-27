<a id="server_storage_api_11"></a>

# Storage API v1.1 (Obsolete)

This document describes the legacy Sync Server Storage API, version 1.1. It has been
superseded by [Sync API v1.5](api-1.5.md).

The Storage server provides web services that can be used to store and
retrieve **Weave Basic Objects** (WBOs) organized into **collections**.

<a id="storage_wbo"></a>

## Weave Basic Object

A **Weave Basic Object (WBO)** is the generic JSON wrapper around all items
passed into and out of the storage server. Like all JSON, WBOs must be UTF-8
encoded. WBOs have the following fields:

| Parameter | Default | Type / Max | Description |
|----------|---------|------------|-------------|
| `id` | required | string (64) | An identifying string. For a user, the id must be unique for a WBO within a collection, though objects in different collections may have the same ID. This **should** be exactly 12 characters from the base64url alphabet. While not enforced by the server, the Firefox client expects this in most cases. |
| `modified` | time submitted | float (2 decimals) | The last-modified date, in seconds since 1970-01-01. Set automatically by the server. |
| `sortindex` | none | integer | Indicates the relative importance of this item in the collection. |
| `payload` | none | string (256k) | A JSON structure encapsulating the data of the record. Defined separately per WBO type. Parts may be encrypted and include decryption metadata. |
| `ttl` | none | integer | Number of seconds to keep this record. After expiration, it will not be returned. |
| `parentid` | none | string (64) | The id of a parent object in the same collection. Used to create hierarchical structures. *(Deprecated)* |
| `predecessorid` | none | string (64) | The id of a predecessor in the same collection. Used to create linked-list-like structures. *(Deprecated)* |

Notes:
- Deprecated fields are likely to be removed in future versions.
- See ECMA-262 for timestamp definition: http://www.ecma-international.org/publications/standards/Ecma-262.htm

### Sample
```json
{
    "id": "-F_Szdjg3GzY",
    "modified": 1278109839.96,
    "sortindex": 140,
    "payload": "{\"ciphertext\":\"e2zLWJYX/iTw3WXQqffo00kuuut0Sk3G7erqXD8c65S5QfB85rqolFAU0r72GbbLkS7ZBpcpmAvX6LckEBBhQPyMt7lJzfwCUxIN/uCTpwlf9MvioGX0d4uk3G8h1YZvrEs45hWngKKf7dTqOxaJ6kGp507A6AvCUVuT7jzG70fvTCIFyemV+Rn80rgzHHDlVy4FYti6tDkmhx8t6OMnH9o/ax/3B2cM+6J2Frj6Q83OEW/QBC8Q6/XHgtJJlFi6fKWrG+XtFxS2/AazbkAMWgPfhZvIGVwkM2HeZtiuRLM=\",\"IV\":\"GluQHjEH65G0gPk/d/OGmg==\",\"hmac\":\"c550f20a784cab566f8b2223e546c3abbd52e2709e74e4e9902faad8611aa289\"}"
}```

## Collections

Each WBO is assigned to a collection with related WBOs. Collection names may
only contain alphanumeric characters, period, underscore, and hyphen.

Default Mozilla collections:

- bookmarks
- history
- forms
- prefs
- tabs
- passwords

Internal-use collections:

- clients
- crypto
- keys
- meta

## URL Semantics

Storage URLs generally follow REST semantics. Request and response bodies are
JSON-encoded.

URL structure:

`https://<server name>/<api pathname>/<version>/<username>/<further instruction>`

| Component | Mozilla Default | Description |
|----------|-----------------|-------------|
| server name | defined by user account | Hostname of the server |
| pathname | none | Prefix associated with the service |
| version | 1.1 | API version |
| username | none | User identifier |
| further instruction | none | Function-specific path |

Certain functions use HTTP Basic Authentication over SSL. If the authentication
username does not match the username in the path, an error response is returned.

## APIs

### GET

`GET /info/collections`

Returns collections and their last-modified timestamps.

`GET /info/collection_usage`

Returns collections and storage usage (KB).

`GET /info/collection_counts`

Returns collections and item counts.

`GET /info/quota`

Returns current usage and quota (KB).

`GET /storage/<collection>`

Returns WBO ids in a collection. Optional parameters:

- ids
- predecessorid (deprecated)
- parentid (deprecated)
- older
- newer
- full
- index_above
- index_below
- limit
- offset
- sort (oldest, newest, index)

Alternate output formats via `Accept` header:

- application/whoisi
- application/newlines

`GET /storage/<collection>/<id>`

Returns the requested WBO.

### PUT

`PUT /storage/<collection>/<id>`

Adds or updates a WBO. Metadata-only update if no payload is provided.
Returns the modification timestamp.

### POST

`POST /storage/<collection>`

Bulk upload of WBOs with a shared timestamp.

Sample response:
```json
{
    "modified": 1233702554.25,
    "success": ["{GXS58IDC}12", "{GXS58IDC}13"],
    "failed": {
    "{GXS58IDC}11": ["invalid parentid"]
    }
}
```

### DELETE

`DELETE /storage/<collection>`

Deletes a collection or selected items.

`DELETE /storage/<collection>/<id>`

Deletes a single WBO.

`DELETE /storage`

Deletes all user records. Requires `X-Confirm-Delete`.

All delete operations return a timestamp.

## Headers

### Retry-After

Used with HTTP 503 to indicate maintenance duration.

### X-Weave-Backoff

Indicates server overload; client should delay sync (usually 1800 seconds).

### X-If-Unmodified-Since

Fails write requests if the collection was modified since the given timestamp.

### X-Weave-Alert

Human-readable warning or informational messages.

### X-Weave-Timestamp

Current server timestamp; also modification time for PUT/POST.

### X-Weave-Records

If supported, returns the number of records in a multi-record GET response.

## HTTP Status Codes

### 200

Request processed successfully.

### 400

Invalid request or data. Response includes a numeric error code.

### 401

Invalid credentials, possibly due to node reassignment or password change.

### 404

Resource not found. Returned for missing records or empty collections.

### 503

Server maintenance or overload. Used with `Retry-After`.
