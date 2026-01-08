<a id="sync_storageformat5"></a>

# Global Storage Version 5

This document describes version 5 of Sync's global storage format. It describes
not only the technical details of the storage format, but also the semantics for
how clients supporting version 5 should interact with the Sync server.

## Overview

A single unencrypted record called the **metaglobal record** (because it exists
in the *meta* collection with the id *global*) stores essential data used to
instruct clients how to behave.

A special record called the **cryptokeys record** (because it exists in the
*crypto* collection with the id *keys*) holds encrypted keys which are used to
encrypt, decrypt, and verify all other encrypted records on the server.

## Cryptography

### Overview

Every encrypted record (and all but one record on the server is encrypted) is
encrypted using symmetric key encryption and verified using HMAC hashing.
The symmetric encryption and HMAC verification keys are only available to
client machines; they are not transmitted to the server in any readable form.
This means that the data on the server cannot be read by anyone with access
to the server.

The symmetric encryption key and HMAC key together form a **key bundle**.
Each key is 256 bits.

Individual records are encrypted with AES-256. The encryption key from a key
bundle is combined with a per-record 16-byte IV and user data is converted into
ciphertext. The ciphertext is then *signed* with the key bundle’s **HMAC key**.
The ciphertext, IV, and HMAC value are uploaded to the server.

When Sync is initially configured by signing in with a Mozilla account, the
client obtains a 256-bit encryption key called the **Class-B Master Key**.
This key is used to derive a special key bundle via HKDF, called the
**Sync Key Bundle**. The Sync Key Bundle is used to encrypt and decrypt a special
record on the server which holds additional key bundles. These bundled keys are
used to encrypt and decrypt all other records on the server.

### Terminology

**Class-B Master Key**  
256-bit encryption key obtained from Mozilla accounts, serving as the root key
for Sync.

**Key Bundle**  
A pair of 256-bit keys: one for symmetric encryption and one for HMAC hashing.

**Sync Key Bundle**  
A Key Bundle derived from the Class-B Master Key via HKDF.

**HKDF**  
Cryptographic technique used to derive keys from another key.

**Bulk Key Bundle**  
A collection of Key Bundles used to secure records, encrypted with the Sync Key
Bundle.

**Cleartext**  
The unencrypted form of user data.

**Ciphertext**  
The encrypted form of cleartext.

**Encryption Key**  
The key used to convert cleartext into ciphertext.

**HMAC Key**  
The key used to verify message integrity.

**Symmetric Encryption**  
Encryption and decryption using the same secret key.

**HMAC Hashing**  
A method to verify that ciphertext has not been tampered with.

## Class-B Master Key

All encryption keys used in Sync are ultimately derived from the
Class-B Master Key, which is managed by Mozilla accounts and obtained through
the Accounts/Sync sign-in protocol (referred to as *kB*).

All clients collaborating via Sync share the same value for this key.
It must never be transmitted to untrusted parties or stored where it can
be accessed by others, including the storage server.

## Sync Key Bundle

The Sync Key Bundle is derived from the Class-B Master Key using SHA-256
HMAC-based HKDF (RFC 5869).

A total of 64 bytes are derived. The first 32 bytes form the encryption key,
and the remaining 32 bytes form the HMAC key.

Pseudo-code:

```python
info = "identity.mozilla.com/picl/v1/oldsync"
prk = HKDF-Extract-SHA256(0x00 * 32, master_key)
okm = HKDF-Expand-SHA256(prk, info, 64)

encryption_key = okm[0:32]
hmac_key = okm[32:64]
```

## Record Encryption

Each record is encrypted using AES-256 in CBC mode and signed using HMAC-SHA256.

Pseudo-code:

```python
cleartext = "SECRET MESSAGE"
iv = randomBytes(16)
ciphertext = AES256(cleartext, bundle.encryption_key, iv)
hmac = HMACSHA256(bundle.hmac_key, base64(ciphertext))
```
The ciphertext, IV, and HMAC are stored in the record payload.

## Record Decryption

When retrieving a record, the client verifies the HMAC before attempting
decryption. If verification fails, the record must not be decrypted.

Pseudo-code:
```python
local_hmac = HMACSHA256(hmac_key, base64(ciphertext))
if local_hmac != record_hmac:
    error

cleartext = AESDecrypt(ciphertext, encryption_key, iv)
```

## Metaglobal Record

The `meta/global` record contains metadata describing server state, including
storage version and enabled engines. It is not encrypted.

Fields include:

- **storageVersion**
- **syncID**
- **engines**
- **declined** (Protocol 1.5)

Example:
```json
{
    "syncID": "7vO3Zcdu6V4I",
    "storageVersion": 5,
    "engines": {
    "clients":   {"version":1,"syncID":"Re1DKzUQE2jt"},
    "bookmarks": {"version":2,"syncID":"ApPN6v8VY42s"}
    },
    "declined": ["passwords"]
}
```

Clients must verify storage version compatibility before modifying data.

## crypto/keys Record

In version 5, all bulk keys are stored in the `crypto/keys` record.
It is encrypted using the Sync Key Bundle.

Fields:

- **default**: default key pair
- **collections**: per-collection key pairs
- **collection**: always `"crypto"`

Each key is Base64-encoded.

## Collection Records

All non-special records store encrypted payloads with:

- **ciphertext**
- **IV**
- **hmac**

Example:

```json
{
    "payload": "{\"ciphertext\":\"...\",\"IV\":\"...\",\"hmac\":\"...\"}",
    "id": "GJN0ojnlXXhU",
    "modified": 1332402035.78
}
```

## Encryption Example

Given cleartext:

```json
{
    "foo": "supersecret",
    "bar": "anothersecret"
}
```

Pseudo-code:
```python
key_pair = bulk_key_bundle.getKeyPair(collection_name)
iv = randomBytes(16)
ciphertext = AES256(cleartext, key_pair.encryption_key, iv)
hmac = HMACSHA256(base64(ciphertext), key_pair.hmac_key)

payload = {
    "ciphertext": base64(ciphertext),
    "IV": base64(iv),
    "hmac": base64(hmac)
}
```

## Decryption Example

Pseudo-code:
```python
fields = JSONDecode(record.payload)
ciphertext_b64 = fields.ciphertext

local_hmac = HMACSHA256(ciphertext_b64, hmac_key)
if local_hmac != remote_hmac:
    error

cleartext = AESDecrypt(Base64Decode(ciphertext_b64), encryption_key, iv)
object = JSONDecode(cleartext)
```
