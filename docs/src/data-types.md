# Firefox Sync data types

The following are important details around each syncing data type.

Users can configure which Firefox data types are synced across all connected Firefoxes. On desktop, this is found on `about:preferences#sync`; on mobile this is found in `Sync Settings` under the application menu. Mobile Firefox currently only shows data types that are supported on that platform. Any changes to selected data types on any platform are applied at the account level: they will take effect on all clients connected by that account.

## Bookmarks

- All bookmarks are synced; users cannot specify bookmarks to include or exclude.
- Bookmarks are merged when synced, so that the result is all bookmarks from all connected devices.
- We do not identify the device that a bookmark came from. However, on mobile you will see a "Desktop Bookmarks" folder, and on desktop you will see a "Mobile bookmarks" folder that the respective platforms use by default.
- There is a record per bookmark in sync storage.

## History

- We sync the most recent 5,000 history entries.
- History is merged when synced, so that the result is the most recent history from all connected devices.
- There is a record per history entry in sync storage.

## Open tabs

- The ability to use the Send Tab feature does not require the user to be syncing tabs.
- The ability to remotely close tabs does require the user to be syncing tabs.
- Unlike other synced data, tabs are synced with an associated client/device identifier; they are not merged.
- Tabs were changed with MR 2022 to sync every 5s after a tab change.
- Synced tab data is a subset of the local tab data. We do not sync:
  - Tabs with any URLs matching these schemes. This includes reader view tabs.
  - Any tabs in private windows (which is an intentional decision).
  - The back/forward stack (i.e. the "back" and "forward" buttons are disabled when opening a remote tab).
  - Window groupings. If you have multiple windows open, each with its own tabs, all your tabs will be flattened into a single list in the Synced Tabs views.
  - Whether the tab is pinned or not. We sync pinned tabs, but other devices see them as regular tabs, and they aren't sorted in any particular order.
  - Top sites from the New Tab page. Pinned top sites are synced completely separately, as part of preferences sync. Frequently visited top sites that aren't pinned rely on the frecency from history, which sometimes will mean they become top sites after history syncs. Manual pinning of top sites are not synced.
  - Additional page state. Cookies, local storage, scroll position, and any text entered in form fields on the page are never synced.
- There is a record per device with tabs in sync storage.

## Logins & passwords (AKA. Logins)

- Logins & passwords are merged when synced, so that the result is all logins & passwords from all connected devices.
- There is a record per login & password entry in sync storage.

## Addresses

- Addresses are only enabled for specific countries/geos.
- Addresses are behind a feature flag on Android.
- There is a record per address entry in sync storage.

## Payment methods (Credit Cards)

- Payment methods are merged when synced, so that the result is all payment methods from all connected devices.
- There is a record per payment method (credit card) in sync storage.

## Add-ons

- Add-ons automatically (for now) sync between desktop clients only.
- Add-ons categorically include web extensions, themes, and language packs, but language packs do not sync.
- Themes do sync, but can be adversely affected by Settings sync (see below).
- Web extensions syncing automatically means that installation/removal or enabling/disabling of an extension from one connected device will result in that same action occurring on the other connected device.
- Web extensions syncing does not imply that the extensions share data; extension's ability to share data is based on the extension developer using the web extensions `storage.sync` API.
- Extensions on any platform cannot use synced storage (i.e. the `storage.sync` API) unless the user has checked the "Add-ons" option in Sync settings/Choose What To Sync.
- There is a record per add-on in sync storage.

## Settings (Prefs)

- Prefs are not merged when synced. They synced as an entire set, and the latest write wins.
- Settings sync between desktop clients only; there is no mobile analogue for desktop preferences, so no mapping exists.
- By "settings" we mean: a grab-bag of things from `about:config` (specifically, anything of the form `services.sync.prefs.sync.*`).
- All of the syncable prefs are synced: users cannot currently choose to sync only a subset of these prefs.
  - Advanced/Adventuresome users can include/exclude certain preferences (see documentation for details).

## Other data

There are four other collections of data that sync. These are special, and they continue syncing even if you uncheck all displayed data types in the Choose What To Sync dialog.

- **clients**: A list of clients, used in reconciling the list of clients received from Accounts.
- **meta**: A list of data that allows the client to coordinate syncing (engines declined, syncID, storageVersion, etc).
- **crypto**: Cryptographic keys and data for encryption/decryption.
- **keys**: Key management data for sync encryption.
