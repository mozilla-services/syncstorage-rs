<a id="howto_run_fxa"></a>

# Run Your Own Mozilla Accounts Server (Outdated)

The Mozilla accounts server is deployed on our systems using RPM packaging,
and we don't provide any other packaging or publish official builds yet.

> **Note:** This guide is preliminary and vastly incomplete. If you have any
> questions or find any bugs, please don't hesitate to file an issue: 
[Syncstorage-rs GitHub Issues](https://github.com/mozilla-services/syncstorage-rs/issues).

> **Note:** You might also be interested in  
> [this Docker-based self-hosting guide](https://github.com/michielbdejong/fxa-self-hosting)  
> (use at your own risk - quite out of date).

The Mozilla accounts server is hosted in **git** and requires **nodejs**.
Make sure your system has these, or install them:

- **git**: <http://git-scm.com/downloads>
- **nodejs**: <http://nodejs.org/download>

A self-hosted Mozilla accounts server requires two components:

- an **auth-server** that manages the accounts database
- a **content-server** that hosts a web-based user interface

Clone the fxa repository [linked here](https://github.com/mozilla/fxa/) and follow the README to deploy your own auth-server and content-server instances. <https://github.com/mozilla/fxa/>


Now direct Firefox to use your servers rather than the default, Mozilla-hosted
ones. The procedure varies a little between desktop and mobile Firefox, and
may not work on older versions of the browser.

---

## Desktop Firefox (version 52 or later)

1. Enter `about:config` in the URL bar.
2. Right-click anywhere on the page and choose **New > String**.
3. Enter `identity.fxaccounts.autoconfig.uri` for the name, and your
   content-server URL for the value.
4. Restart Firefox for the change to take effect.

> **Note:** This must be set prior to loading the sign-up or sign-in page
> in order to take effect, and its effects are reset on sign-out.

---

## Firefox for iOS (version 9.0 or later)

1. Go to **Settings**.
2. Tap on the **Version number** 5 times.
3. Tap **Advance Account Settings**.
4. Enter your content-server URL.
5. Toggle **Use Custom Account Service** to on.

---

## Firefox Preview for Android (“Fenix”)

- There is not yet support for using a non-Mozilla-hosted account server.
- Work is being tracked in this GitHub issue:  
  <https://github.com/mozilla-mobile/fenix/issues/3762>

---

## Firefox for Android (“Fennec”, version 44 or later)

1. Enter `about:config` in the URL bar.
2. Search for items containing `fxaccounts`, and edit them to use your
   self-hosted URLs.

### Auth server

Use your auth-server URL to replace `api.accounts.firefox.com` in:

- `identity.fxaccounts.auth.uri`

### Content server

Use your content-server URL to replace `accounts.firefox.com` in:

- `identity.fxaccounts.remote.webchannel.uri`
- `webchannel.allowObject.urlWhitelist`

### Optional: OAuth and profile servers

Use your OAuth and profile server URLs to replace
`{oauth,profile}.accounts.firefox.com` in:

- `identity.fxaccounts.remote.profile.uri`
- `identity.fxaccounts.remote.oauth.uri`

> **Important:** *After* creating the Android account, changes to
> `identity.fxaccounts` prefs will be *ignored*.  
> If you need to change the prefs, delete the Android account using
> **Settings > Sync > Disconnect…**, update the pref(s), and sign in again.
>
> Non-default Mozilla account URLs are displayed in the
> **Settings > Sync** panel in Firefox for Android, so you should be able
> to verify your URL there.

---

Since the Mozilla-hosted sync servers will not trust assertions issued by
third-party accounts servers, you will also need to run your own
sync-1.5 server. See [How To Run Your Own Sync-1.5 Server](./how-to-run-sync-server.md).

Please note that the `fxa-content-server` repository includes graphics and
other assets that make use of Mozilla trademarks. If you are doing anything
other than running unmodified copies of the software for personal use, please
review:

- Mozilla Trademark Policy:  
  <https://www.mozilla.org/en-US/foundation/trademarks/policy/>
- Mozilla Branding Guidelines:  
  <http://www.mozilla.org/en-US/styleguide/identity/mozilla/branding/>

You can ask for help on Matrix (chat.mozilla.org) in the **#fxa** room:  
<https://chat.mozilla.org/#/room/#fxa:mozilla.org>

---

### Additional reading

- [How to connect Firefox for Android to self-hosted Mozilla account and Firefox Sync servers](http://www.ncalexander.net/blog/2014/07/05/how-to-connect-firefox-for-android-to-self-hosted-services/)
