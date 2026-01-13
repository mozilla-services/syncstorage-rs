<a id="server_storage_api_10"></a>

# Storage API v1.0 (Obsolete)

This document describes the legacy Sync Server Storage API, version 1.0. It has been
superseded by [Sync API v1.5](api-1.5.md).

## Weave Basic Object (WBO)

A Weave Basic Object is the generic wrapper around all items passed into and
out of the Weave server. The Weave Basic Object has the following fields:

| Parameter | Default | Max | Description |
|----------|---------|-----|-------------|
| id | required | 64 | An identifying string. For a user, the id must be unique for a WBO within a collection, though objects in different collections may have the same ID. Ids should be ASCII and not contain commas. |
| parentid | none | 64 | The id of a parent object in the same collection. This allows for the creation of hierarchical structures (such as folders). |
| predecessorid | none | 64 | The id of a predecessor in the same collection. This allows for the creation of linked-list-esque structures. |
| modified | time submitted | float (2 decimal places) | The last-modified date, in seconds since 1970-01-01 (UNIX epoch time). Set by the server. |
| sortindex | none | 256K | A string containing a JSON structure encapsulating the data of the record. This structure is defined separately for each WBO type. Parts of the structure may be encrypted, in which case the structure should also specify a record for decryption. |
| payload | none | 256K | The record payload. |

Reference: http://www.ecma-international.org/publications/standards/Ecma-262.htm

Weave Basic Objects and all data passed into the Weave Server should be UTF-8 encoded.

### Sample
```json
{
    "id": "B1549145-55CB-4A6B-9526-70D370821BB5",
    "parentid": "88C3865F-05A6-4E5C-8867-0FAC9AE264FC",
    "modified": "2454725.98",
    "payload": "{\"encryption\":\"http://server/prefix/version/user/crypto-meta/B1549145-55CB-4A6B-9526-70D370821BB5\", \"data\": \"a89sdmawo58aqlva.8vj2w9fmq2af8vamva98fgqamff...\"}"
}
```

## Collections

Each WBO is assigned to a collection with other related WBOs. Collection names
may only contain alphanumeric characters, period, underscore and hyphen.

Collections supported at this time are:

- bookmarks
- history
- forms
- prefs
- tabs
- passwords

Additionally, the following collections are supported for internal Weave client use:

- clients
- crypto
- keys
- meta

## URL Semantics

Weave URLs follow, for the most part, REST semantics. Request and response
bodies are all JSON-encoded.

The URL for Weave Storage requests is structured as follows:

`https://<server name>/<api pathname>/<version>/<username>/<further instruction>`

| Component | Mozilla Default | Description |
|----------|-----------------|-------------|
| server name | defined by user account node | the hostname of the server |
| pathname | none | the prefix associated with the service on the box |
| version | 1.0 | The API version. May be integer or decimal |
| username | none | The name of the object (user) to be manipulated |
| further instruction | none | The additional function information as defined in the paths below |

Weave uses HTTP basic auth (over SSL). If the auth username does not match the
username in the path, the server will issue an error response.

The Weave API has a set of Weave Response Codes to cover errors in the request
or on the server side.

## GET

### info/collections

`GET /<version>/<username>/info/collections`

Returns a hash of collections associated with the account, along with the last
modified timestamp for each collection.

### info/collection_counts

`GET /<version>/<username>/info/collection_counts`

Returns a hash of collections associated with the account, along with the total
number of items for each collection.

### info/quota

`GET /<version>/<username>/info/quota`

Returns a tuple containing the user's current usage (in K) and quota.

### storage/collection

`GET /<version>/<username>/storage/<collection>`

Returns a list of the WBO ids contained in a collection.

Optional parameters:

- ids
- predecessorid
- parentid
- older
- newer
- full
- index_above
- index_below
- limit
- offset
- sort (oldest, newest, index)

### storage/collection/id

`GET /<version>/<username>/storage/<collection>/<id>`

Returns the WBO in the collection corresponding to the requested id.

## Alternate Output Formats

Triggered by the Accept header:

- application/whoisi: each record consists of a 32-bit integer defining the
  length of the record, followed by the JSON record
- application/newlines: each record is a separate JSON object on its own line;
  newlines in the body are replaced by \u000a

## APIs

### PUT

`PUT /<version>/<username>/storage/<collection>/<id>`

Adds or updates a WBO. Without a payload, only metadata fields are updated.

Returns the modification timestamp.

### POST

`POST /<version>/<username>/storage/<collection>`

Takes an array of WBOs and performs atomic PUTs with a shared timestamp.

Example response:
```json
    {
      "modified": 1233702554.25,
      "success": ["{GXS58IDC}12","{GXS58IDC}13"],
      "failed": {
        "{GXS58IDC}11": ["invalid parentid"]
      }
    }
```

### DELETE

`DELETE /<version>/<username>/storage/<collection>`

Deletes the collection or selected items.

`DELETE /<version>/<username>/storage/<collection>/<id>`

Deletes a single WBO.

`DELETE /<version>/<username>/storage`

Deletes all records for the user. Requires X-Confirm-Delete.

All delete operations return a timestamp.

## General Weave Headers

### X-Weave-Backoff

Indicates server overload. Client should retry after the specified seconds.

### X-If-Unmodified-Since

Fails write requests if the collection has changed since the given timestamp.

### X-Weave-Alert

Human-readable warnings or informational messages.

### X-Weave-Timestamp

Server timestamp; also the modification time for PUT/POST requests.

### X-Weave-Records

If supported, returns the number of records in a multi-record GET response.
