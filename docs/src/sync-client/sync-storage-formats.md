<a id="sync_storageformats"></a>

# Sync Storage Formats

The way that Sync clients store data on a storage server is defined by sets of
integer **storage versions**. Each storage version defines specific semantics
for how clients are supposed to behave.

## Global Storage Version

There exists a **global storage version** that defines global semantics. This
global version typically specifies:

- What special records exist on the server and what they contain
- The payload format of encrypted records on the server
- How cryptography of data works

Each Sync client is coded to support one or more global storage formats. If a
client encounters a storage format it does not support, it should generally
stop attempting to consume data.

Under no normal circumstances should a client modify data on a server that is
defined with an unknown or newer storage format. Even if an older client wipes
all server data and uploads data using its own format, newer clients may
transparently upgrade the server data to the storage format they support.

Because changing storage formats can prevent some clients from syncing—since
not all clients may be upgraded at the same time—new global storage versions
are introduced very rarely.

## Versions 1, 2, and 3

These versions were used by an older version of Sync that was deprecated in
early 2011.

Historical information is available [here](https://wiki.mozilla.org/index.php?title=Labs/Weave/Developer/Crypto&oldid=200527)

These versions should no longer be in active use and should all be upgraded to
a newer storage format.

## Version 4

This version introduced a new cryptographic model based fully on AES.
Due to a faulty implementation of the cryptography, version 5 was created to
force alpha clients created with the faulty implementation to upgrade.

As a result, version 4 and version 5 are practically identical in design.

## Version 5 (Spring 2011 – Current)

Version 5 replaces version 3’s cryptographic model with one based purely on AES.

A full overview of this format is available in [Global Storage Version 5](global-storage-v5.md)

Historical notes are available [here](https://wiki.mozilla.org/index.php?title=Services/Sync/SimplifiedCrypto&oldid=276735)

## Collection / Object Format Versions

The formats of unencrypted records stored on the server are also versioned.
For example, records in the *bookmarks* collection are all defined to be of a
specific object format version.

Strictly speaking, these versions are tied to a specific global storage version.
However, since all storage formats to date have stored the per-collection
version in a special record, these object format versions effectively apply
across all global storage versions.

These formats are fully documented in [Firefox Object Formats](fx-object-formats.md).