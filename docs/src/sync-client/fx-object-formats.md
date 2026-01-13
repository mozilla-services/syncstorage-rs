<a id="sync_objectformats"></a>

# Firefox object formats

Decrypted data objects are cleartext JSON strings.

Each collection can have its own object structure. This document describes the
format of each collection.

The object structure is versioned with the version metadata stored in the
`meta/global` payload.

The following sections, named by the corresponding collection name, describe
the various object formats and how they’re used. Note that object structures
may change in the future and may not be backwards compatible.

In addition to these custom collection object structures, the Encrypted
DataObject adds fields like *id* and *deleted*. Also remember that there is
data at the Weave Basic Object (WBO) level as well as *id*, *modified*,
*sortindex* and *payload*.

## Add-ons

### Version 1

Version 1 is likely only affiliated with storage format 5 clients.

- **addonID** (*string*): Public identifier of add-on. This is the *id*
  attribute from an Addon object obtained from the AddonManager.
- **applicationID** (*string*): The application ID the add-on belongs to.
- **enabled** (*bool*): Indicates whether the add-on is enabled or disabled.
  `true` means enabled.
- **source** (*string*): Where the add-on came from. *amo* means it came from
  addons.mozilla.org or a trusted site.

## Bookmarks

### Version 1

One bookmark record exists for each *bookmark item*, where an item may actually
be a folder or a separator. Each item will have a *type* that determines what
other fields are available in the object. The following sections describe the
object format for a given *type*.

Each bookmark item has a *parentid* and *predecessorid* to form a structure
like a tree of linked-lists to provide a hierarchical ordered list of
bookmarks, folders, etc.

#### bookmark

This describes a regular bookmark that users can click to view a page.

- **title** (*string*): name of the bookmark
- **bmkUri** (*string*): uri of the page to load
- **description** (*string*): extra description if provided
- **loadInSidebar** (*boolean*): true if the bookmark should load in the sidebar
- **tags** (*array of strings*): tags for the bookmark
- **keyword** (*string*): alias to activate the bookmark from the location bar
- **parentid** (*string*): GUID of the containing folder
- **parentName** (*string*): name of the containing folder
- **predecessorid** (*string*): GUID of the item before this (empty if it’s first)
- **type** (*string*): `"bookmark"`

#### microsummary

Microsummaries allow pages to be summarized for viewing from the toolbar. This
extends *bookmark*, so the usual *bookmark* fields apply.

Reference: https://developer.mozilla.org/en/Microsummary_topics

- **generatorUri** (*string*): uri that generates the summary
- **staticTitle** (*string*): title to show when no summaries are available
- **title** (*string*): name of the microsummary
- **bmkUri** (*string*): uri of the page to load
- **description** (*string*): extra description if provided
- **loadInSidebar** (*boolean*): true if the bookmark should load in the sidebar
- **tags** (*array of strings*): tags for the bookmark
- **keyword** (*string*): alias to activate the bookmark from the location bar
- **parentid** (*string*): GUID of the containing folder
- **parentName** (*string*): name of the containing folder
- **predecessorid** (*string*): GUID of the item before this (empty if it’s first)
- **type** (*string*): `"microsummary"`

#### query

Place queries are special bookmarks with a `place:` uri that links to an
existing folder/tag. This extends *bookmark*, so the usual *bookmark* fields
apply.

- **folderName** (*string*): name of the folder/tag to link to
- **queryId** (*string*, optional): identifier of the smart bookmark query
- **title** (*string*): name of the query
- **bmkUri** (*string*): `place:` uri query
- **description** (*string*): extra description if provided
- **loadInSidebar** (*boolean*): true if the bookmark should load in the sidebar
- **tags** (*array of strings*): tags for the query
- **keyword** (*string*): alias to activate the bookmark from the location bar
- **parentid** (*string*): GUID of the containing folder
- **parentName** (*string*): name of the containing folder
- **predecessorid** (*string*): GUID of the item before this (empty if it’s first)
- **type** (*string*): `"query"`

#### folder

Folders contain bookmark items like bookmarks and other folders.

- **title** (*string*): name of the folder
- **parentid** (*string*): GUID of the containing folder
- **parentName** (*string*): name of the containing folder
- **predecessorid** (*string*): GUID of the item before this (empty if it’s first)
- **type** (*string*): `"folder"`

#### livemark

Livemarks act like folders with a dynamic list of bookmarks, e.g. an RSS feed.
This extends *folder*, so the usual *folder* fields apply.

Reference: https://developer.mozilla.org/en/Using_the_Places_livemark_service

- **siteUri** (*string*): site associated with the livemark
- **feedUri** (*string*): feed to get items for the livemark
- **title** (*string*): name of the livemark
- **parentid** (*string*): GUID of the containing folder
- **parentName** (*string*): name of the containing folder
- **predecessorid** (*string*): GUID of the item before this (empty if it’s first)
- **type** (*string*): `"livemark"`

#### separator

Separators help split sections of a folder.

- **pos** (*string*): position (index) of the separator
- **parentid** (*string*): GUID of the containing folder
- **parentName** (*string*): name of the containing folder
- **predecessorid** (*string*): GUID of the item before this (empty if it’s first)
- **type** (*string*): `"separator"`

### Version 2

Same as engine version 1, except:

- the `predecessorid` is removed from all records;
- instead folder and livemark records have a `children` attribute which is an
  array of child GUIDs in order of their appearance in the folder:
  - **children** (*array of strings*): ordered list of child GUIDs
- the special folders `menu` and `toolbar` now have records that are synced,
  purely to maintain order within them according to their `children` array.
- **dateAdded** (*unix timestamp*): The best lower bound on the creation date
  for this record we have. May be missing, in the case of records uploaded by
  older clients when no newer client is available to fix it up.

### Version 3

> **Note:** Proposal corresponding with storage format 6.

Same as version 2 except:

- Support for microsummaries is removed
- We use the ASCII URL

TODO: document full format here since diffs are inconvenient to read.

## Clients

### Version 1

Client records identify a user’s one or multiple clients that are accessing the
data. The existence of client records can change the behavior of the Firefox
Sync client — multiple clients and/or mobile clients result in syncs to happen
more frequently.

- **name** (*string*): name of the client connecting
- **type** (*string*): type of the client: `"desktop"` or `"mobile"`
- **commands** (*array*): commands to be executed upon next sync — see below for more

In Protocol 1.5, client records additionally include:

- **version** (*string*): a version indicator for this client, such as `"29.0a1"`. Optional.
- **protocols** (*array*): an array of Sync protocol versions supported by this client, such as `["1.1", "1.5"]`. Optional.

In Bug 1097222 additional optional fields were added:

- **os** (*string*): an OS name, most likely one of `"Darwin"` (Mac OS X),
  `"WINNT"` (Windows), `"Android"`, or `"iOS"`.
- **appPackage** (*string*): an unambiguous identifier for the client application.
  For Android, this is the package (e.g., **org.mozilla.firefox_beta**). For desktop
  this is the value of **Services.appinfo.ID**.
- **application** (*string*): a human-readable application name, such as `"Nightly"`
  or `"Firefox"`.
- **formfactor** (*string*): a value such as `"phone"`, `"tablet"` (or the more
  specific `"largetablet"`, `"smalltablet"`), `"desktop"`, `"laptop"`, `"tv"`.
- **device** (*string*): a description of the hardware that this client uses.
  Currently only supported by Android; returns values like `"HTC One"`.

If these fields are missing, clients are expected to fall back to behaviors
that do not depend on the missing data.

Clients should preserve existing fields if possible when sending commands to
another client.

#### commands

`commands` is an array of JSON objects. Each element has the following attributes:

- **command** (**string**): The name of the command to execute. Currently
  supported commands include `"resetAll"`, `"resetEngine"`, `"wipeAll"`,
  `"wipeEngine"`, `"logout"`, `"displayURI"`, `"repairRequest"` and
  `"repairResponse"`, although not all commands are supported by all implementations.
- **args** (**array of strings/objects**): Arguments for the command. These
  are specific to the command.
- **flowIID** (*optional, string*): A guid used for reporting telemetry. Both
  the sender and receiver of the command should report this ID in telemetry so
  the reliability of the sending and reception of the command can be tracked.

### Version 2 (never deployed)

> **Note:** Proposal corresponding with storage format 6.

Each client has its own record which it is authoritative for. No other client
should modify another client’s record except in the case where records are
deleted.

The payload of a client record has the following fields:

- **name** (*string*): The name of the client. This is a user-facing value and
  may be provided by the user.
- **formfactor** (*string*): The form factor of the client. Recognized values
  include *phone*, *tablet*, *laptop*, *desktop*.
- **application** (*string*): String identifying the application behind the
  client. This should only be used for presentation purposes (e.g. choosing what
  logo to display).
- **version** (*string*): The version of the client. This is typically the
  version of the application. Again, this should only be used for presentation
  purposes.
- **capabilities** (*object*): Denotes the capabilities a client possesses.
  Keys are string capability names. Values are booleans indicating whether the
  capability is enabled. Modifying the capabilities of another client’s record
  should not change the enabled state on that client.
- **mpEnabled** (*bool*): Whether *master password* is enabled on the client. If
  *master password* is enabled on any client in an account, the current client
  should hesitate before downloading passwords if *master password* is not
  enabled locally, as this would decrease the security of the passwords locally
  since they wouldn’t be protected with a *master password*.

## Commands

### Version 1

> **Note:** Proposal corresponding with storage format 6.

This collection holds commands for clients to process. The ID of command records
is randomly generated.

Command records contain an extra unencrypted field in the BSO that says which
client ID they belong to. The value is the hash of the client ID with the
commands engine salt.

Command data is an object with the following fields:

- **receiverID** (*string*): Client ID of the client that should receive the
  command. This is duplicated inside the payload so it can be verified by the
  HMAC.
- **senderID** (*string*): Client ID of the client that sent the command.
- **created** (*number*): Integer seconds since Unix epoch that command was
  created.
- **action** (*string*): The action to be performed by the command. Each command
  has its own name that uniquely identifies it. It is recommended that actions
  be namespaced using colon-delimited notation. Sync’s commands are all prefixed
  with *sync:* (e.g. **sync:wipe**). If a command is versioned, the name is the
  appropriate place to convey that versioning.
- **data** (*object*): Additional data associated with command. This is
  dependent on the specific command type being issued.

## Forms

Form data is used to give suggestions for autocomplete for a HTML text input
form. One record is created for each form entry.

- **name** (*string*): name of the HTML input field
- **value** (*string*): value to suggest for the input

## History

### Version 1

Every page a user visits generates a history item/page. One history (page) per record.

- **histUri** (*string*): uri of the page
- **title** (*string*): title of the page
- **visits** (*array of objects*): a number of how and when the page was visited
- **date** (*integer*): datetime of the visit
- **type** (*integer*): transition type of the visit

Reference: https://developer.mozilla.org/en/nsINavHistoryService#Constants

### Version 2 (never deployed)

> **Note:** Proposal corresponding with storage format 6.

History visits are now stored as a timeline/stream of visits. The historical
information for a particular site/URL is spread out over N>=1 records.

Payloads have the structure:
```json
    {
      "items": [
        "uri": "http://www.mozilla.org/",
        "title": "Mozilla",
        "visits": {
          1: [1340757179.82, 184],
          2: [1340341244.31, 12, 4]
        }
      ]
    }
```

The bulk of the payload is a list of history items. Each item is both a place
and a set of visits.

- **uri** (*string*): URI of the page that was visited.
- **title** (*string*): Title of the page that was visited.
- **visits** (*object*): Mapping of visit type to visit times.

The keys in **visits** define the transition type for the visit. They can be:

- **1**: A link was followed.
- **2**: The URL was typed by the user.
- **3**: The user followed a bookmark.
- **4**: Some inner content was loaded.
- **5**: A permanent redirect was followed.
- **6**: A temporary redirect was followed.
- **7**: The URL was downloaded.
- **8**: User follows a link that was in a frame.

These correspond to nsINavHistoryService’s transition type constants:
https://developer.mozilla.org/en/nsINavHistoryService#Constants

The values for each visit type are arrays which encode the visit time. The
initial element is the wall time of the first visit in seconds since epoch,
typically with millisecond resolution. Each subsequent value is the number of
seconds elapsed since the previous visit. The values: `[100000000.000, 10.100, 5.200]`

Correspond to the times:
```bash
    100000000.000
    100000010.100
    100000015.300
```

The use of deltas to represent times is to minimize serialized size of visits.

## Passwords

Saved passwords help users get back into websites that require a login such as
HTML input/password fields or HTTP auth.

- **hostname** (*string*): hostname that password is applicable at
- **formSubmitURL** (*string*): submission url (GET/POST url set by `<form>`)
- **httpRealm** (*string*): the HTTP Realm for which the login is valid; if not
  provided by the server, the value is the same as hostname
- **username** (*string*): username to log in as
- **password** (*string*): password for the username
- **usernameField** (*string*): HTML field name of the username
- **passwordField** (*string*): HTML field name of the password

If possible, clients should also write fields corresponding to nsILoginMetaInfo:

- **timeLastUsed** (*unsigned long*): local Unix timestamp in milliseconds at which
  this password was last used.
  Note that client clocks can be wrong, and thus this time can be dramatically
  earlier or later than the modified time of the record. Consuming clients should
  be careful to handle out of range values.
- **timeCreated** (*unsigned long*): as with **timeLastUsed**, but for creation.
- **timePasswordChanged** (*unsigned long*): as with **timeLastUsed**, but for password change.
- **timesUsed** (*unsigned long*): the number of uses of this password.

These fields are optional; clients should expect them to be missing. Clients that
don’t use this data locally are encouraged to pass through when it makes sense
(**timeCreated**), or wipe when invalidation is the best option (e.g.,
**timePasswordChanged**).

Clients should use judgment when updating these fields; it’s typically not feasible
to upload new records each time a password is used. During download, a non-matching
timestamp (or missing field) in an otherwise matching local record should not
automatically be treated as a collision. Handling these fields introduces additional
complexities in reconciliation.

The Firefox desktop client began recording this data in Bug 555755.

## Preferences

### Version 1

Some preferences used by Firefox will be synced to other clients. There is only
one record for preferences with a GUID `"preferences"`.

- **value** (*array of objects*): each object describes a preference entry
- **name** (*string*): full name of the preference
- **type** (*string*): type of preference (`int`, `string`, `boolean`)
- **value** (*depends on type*): value of the preference

### Version 2

There is only one record for preferences, using `nsIXULAppInfo.ID` as the GUID.
Custom preferences can be synced by following these instructions:
https://developer.mozilla.org/en/Firefox_Sync/Syncing_custom_preferences

- **value** (*object*): containing name and value of the preferences.

Note: The preferences that determine which preferences are synced are now included as well.

## Tabs

### Version 1

Tabs describe the opened tabs on a given client to provide functionality like
get-up-n-go. Each client will provide one record.

- **clientName** (*string*): name of the client providing these tabs
- **tabs** (*array of objects*): each object describes a tab
- **title** (*string*): title of the current page
- **urlHistory** (*array of strings*): page urls in the tab’s history
- **icon** (*string*): favicon uri of the tab
- **lastUsed** (*integer*): Time in seconds since Unix epoch at which the tab was last accessed.
  Preferred format is an integer, but older clients may write floats or stringified floats,
  so clients should be prepared to receive those formats too.

### Version 2

> **Note:** Proposal corresponding with storage format 6.

In version 2, each tab is represented by its own record (a change from version 1).

Payload fields:

- **clientID** (*string*): ID of the client this tab originated on.
- **title** (*string*): Title of page that is active in the tab.
- **history** (*array of strings*): URLs in this tab’s history. Initial element is the current URL.
  Subsequent URLs were previously visited.
- **lastUsed** (*number*): Time in seconds since Unix epoch that tab was last active.
- **icon** (*string*): Base64 encoded favicon image.
- **groupName** (*string*): Name of tab group this tab is associated with; usually for presentation
  and typically the same across records in a given tab group.
