# Token Server API v1.0

Unless stated otherwise, all APIs are using application/json for the requests
and responses content types.


**GET** **/1.0/<app_name>/<app_version>**

Asks for new token given some credentials in the Authorization header.

By default, the authentication scheme is Mozilla Accounts OAuth 2.0
but other schemes can
potentially be used if supported by the login server.

- **app_name** is the name of the application to access, like **sync**.
- **app_version** is the specific version number of the api that you want
    to access.

The first /1.0/ in the URL defines the version of the authentication
token itself.

Example for Mozilla Account OAuth 2.0::
```
    GET /1.0/sync/1.5
    Host: token.services.mozilla.com
    Authorization: bearer <assertion>
```

This API returns several values in a json mapping:

- **id** -- a signed authorization token, containing the
    user's id for the application and the node.
- **key** -- a secret derived from the shared secret
- **uid** -- the user id for this service
- **api_endpoint** -- the root URL for the user for the service.
- **duration** -- the validity duration of the issued token, in seconds.

Example::
```
HTTP/1.1 200 OK
Content-Type: application/json

{'id': <token>,
    'key': <derived-secret>,
    'uid': 12345,
    'api_endpoint': 'https://db42.sync.services.mozilla.com/1.5/12345',
    'duration': 300,
}
```

If the **X-Client-State** header is included in the request, the
server will compare the submitted value to any previously-seen value.
If it has changed then a new uid and api_endpoint are generated, in
effect "resetting" the node allocation for this user.


## Request Headers


**X-Client-State**

An optional string that can be sent to identify a unique configuration
of client-side state.  It may be up to 32 characters long, and must
contain only characters from the urlsafe-base64 alphabet (i.e.
alphanumeric characters, underscore and hyphen) and the period.

A change in the value of this header may cause the user's node
allocation to be reset, keeping in mind Sync currently has a single node.
Clients should include any client-side state
that is necessary for accessing the selected app.  For example, clients
accessing :ref:`server_syncstorage_api_15` would include a hex-encoded
hash of the encryption key in this header, since a change in the encryption
key will make any existing data unreadable.

Updated values of the **X-Client-State** will be rejected with an error
status of **"invalid-client-state"** if:

* The proposed new value is in the server's list of previously-seen
client-state values for that user.
* The client-state is missing or empty, but the server has previously
seen a non-empty client-state for that user.
* The user's IdP provides generation numbers in their identity
certificates, and the changed client-state value does not correspond
to an increase in generation number.


## Response Headers

**Retry-After**

When sent together with an HTTP 503 status code, this header signifies that
the server is undergoing maintenance. The client should not attempt any
further requests to the server for the number of seconds specified in
the header value.

**X-Backoff**

This header may be sent to indicate that the server is under heavy load
but is still capable of servicing requests.  Unlike the **Retry-After**
header, **X-Backoff** may be included with any type of response, including
a **200 OK**.

Clients should avoid unnecessary requests to the server for the number of seconds
specified in the header value.  For example, clients may avoid pre-emptively
refreshing token if an X-Backoff header was recently seen.

**X-Timestamp**

This header will be included with all "200" and "401" responses, giving
the current POSIX timestamp as seen by the server, in seconds.  It may
be useful for client to adjust their local clock when generating authorization
assertions.


## Error Responses
===============

All errors are also returned, wherever possible, as json responses following the
structure `described in Cornice
<https://cornice.readthedocs.io/en/latest/validation.html#dealing-with-errors>`_.

In cases where generating such a response is not possible (e.g. when a request
if so malformed as to be unparsable) then the resulting error response will
have a *Content-Type* that is not **application/json**.

The top-level JSON object in the response will always contain a key named
`status`, which maps to a string identifying the cause of the error.  Unexpected
errors will have a `status` string of "error"; errors expected as part of
the protocol flow will have a specific `status` string as detailed below.

Error status codes and their corresponding output are:

- **404** : unknown URL, or unsupported application.
- **400** : malformed request. Possible causes include a missing
  option, bad values or malformed json.
- **401** : authentication failed or protocol not supported.
  The response in that case will contain WWW-Authenticate headers
  (one per supported scheme) and may report the following `status`
  strings:

    - **"invalid-credentials"**: authentication failed due to invalid
      credentials e.g. a bad signature on the Authorization assertion.
    - **"invalid-timestamp"**: authentication failed because the included
      timestamp differed too greatly from the server's current time.
    - **"invalid-generation"**:  authentication failed because the server
      has seen credentials with a more recent generation number.
    - **"invalid-client-state"**:  authentication failed because the server
      has seen an updated value of the *X-Client-State* header.
    - **"new-users-disabled"**:  authentication failed because the user has
      not been seen previously on this server, and new user accounts have
      been disabled in the application config.

- **405** : unsupported method
- **406** : unacceptable - the client asked for an Accept we don't support
- **503** : service unavailable (ldap or snode backends may be down)