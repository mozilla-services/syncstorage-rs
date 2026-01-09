# User Flow

**Please Note**: BrowserID has been removed from Mozilla Accounts, and therefore
has also been removed from later versions of Tokenserver. Discussion of
BrowserID presented here is for historic purposes only.

Here's the proposed two-step flow (with BrowserID / Mozilla account assertions):

1. The client trades a BrowserID assertion for an **Auth Token** and
   corresponding secret.
2. The client uses the auth token to sign subsequent requests using
   **Hawk Auth**.

---

## Getting an Auth Token

Sequence diagram (historical):
```bash
Client -> Login Server: request token [1]
Login Server -> BID: verify [2]
Login Server <- BID
Login Server -> User DB: get node [3]
Login Server <- User DB: return node
Login Server -> Node Assignment Server: assign node [4]
Login Server <- Node Assignment Server: return node
Login Server -> Login Server: create response [5]
Client <- Login Server: token [6]
```

---

## Calling the Service

Sequence diagram:
```bash
Client -> Client: sign request [7]
Client -> Service Node: perform request [8]
Service Node -> Service Node: verify token and signature [9], [10]
Service Node -> Service Node: process request [11]
Client <- Service Node: response
```

---

## Detailed Steps

- The client requests a token, providing its BrowserID assertion [1]:
```bash
GET /1.0/sync/request_token HTTP/1.1
Host: token.services.mozilla.com
Authorization: Browser-ID <assertion>
```

- The **Login Server** checks the BrowserID assertion [2].  
  This step is performed locally without calling an external BrowserID server,
  although this could potentially happen. The server may use PyBrowserID along
  with the BID.org certificate.

  The user's email address is extracted, along with any **Generation Number**
  associated with the BrowserID certificate.

- The **Login Server** queries the **User DB** for an existing record matching
  the user's email address.

  If found, the allocated **Node** and the previously seen **Generation Number**
  are returned.

- If the submitted **Generation Number** is smaller than the recorded one, the
  **Login Server** returns an error because the client's BrowserID credentials
  are out of date.

  If the submitted **Generation Number** is larger than the recorded one, the
  **Login Server** updates the Users DB with the new value.

- If the user is not yet allocated to a **Node**, the **Login Server** requests
  one from the **Node Assignment Server** [4].

- The **Login Server** creates a response containing an **Auth Token** and a
  corresponding **Token Secret** [5], and sends it back to the client.

  - The **Auth Token** contains the user ID and a timestamp, and is signed using
    the **Signing Secret**.
  - The **Token Secret** is derived from the **Master Secret** and the
    **Auth Token** using **HKDF**.
  - The **Node** URL is included in the response as `api_endpoint` [6].
```bash
HTTP/1.1 200 OK
Content-Type: application/json

{
'id': <token>,
'secret': <derived-secret>,
'uid': 12345,
'api_endpoint': 'https://example.com/app/1.0/users/12345'
}
```

- The client saves the node location and Hawk authentication parameters for use
  in subsequent requests [6].

- For each subsequent request to the **Service**, the client computes an
  `Authorization` header using **Hawk Auth** [7] and sends the request to the
  allocated node [8]:
```bash
POST /request HTTP/1.1
Host: some.node.services.mozilla.com
Authorization: Hawk id=<auth-token>
                    ts="137131201"
                    nonce="7d8f3e4a"
                    mac="bYT5CMsGcbgUdFHObYMEfcx6bsw="
```

- The service node uses the **Signing Secret** to validate the **Auth Token** [9].
  If the token is invalid or expired, the node returns `401 Unauthorized`.

- The node derives the **Token Secret** from its **Master Secret** and the
  **Auth Token**, and verifies the request signature [10]. If invalid, it
  returns `401 Unauthorized`.

- The node processes the request as defined by the **Service** [11].
