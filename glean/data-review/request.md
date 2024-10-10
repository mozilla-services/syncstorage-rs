
# Request for data collection review form
# Syncstorage: Daily Active User Count

**All questions are mandatory. You must receive review from a data steward peer on your responses to these questions before shipping new data collection.**

1) What questions will you answer with this data?

The objective is to measure Daily Active Users for Sync.  This is to be an internal metric, emitted from the server of the application.

2) Why does Mozilla need to answer these questions?  Are there benefits for users? Do we need this information to address product or business requirements? Some example responses:

* We need to understand usage patterns of Sync from the server.
* This is necessary to establish a baseline measurement of Daily, Weekly, and Monthly usage of Sync. Essential KPI metric for Sync.


3) What alternative methods did you consider to answer these questions? Why were they not sufficient?

* Previously, this metric was measured by FxA/Mozilla Accounts. Logins were associated with an OAuth Client ID. However, as Relay and other account recovery tools make use of Firefox accounts for their auth, this will alter the reliability of this metric. A user could also make use of the aformentioned services without Sync being enabled, resulting in further inaccuracies.
* A decision was made to move away from attributing active use through auth tokens towards a metric that counts each time Sync collections are accessed.

4) Can current instrumentation answer these questions?

* It does at the current moment, but in a short time will no longer be able to. 

5) List all proposed measurements and indicate the category of data collection for each measurement, using the [Firefox data collection categories](https://wiki.mozilla.org/Data_Collection) found on the Mozilla wiki.   

**Note that the data steward reviewing your request will characterize your data collection based on the highest (and most sensitive) category.**

<table>
  <tr>
    <td>Measurement Name</td>
    <td>Measurement Description</td>
    <td>Data Collection Category</td>
    <td>Tracking Bug #</td>
  </tr>
  <tr>
    <td>sync_event</td>
    <td>Event to record an instance of sync backend activity initiated by client.</td>
    <td>Cat 2: Interaction Data</td>
    <td><a href="https://bugzilla.mozilla.org/show_bug.cgi?id=1923967">https://bugzilla.mozilla.org/show_bug.cgi?id=1923967</a></td>
  </tr>
  <tr>
    <td>hashed_fxa_uid</td>
    <td>Sync user identifier. Uses `hashed_fxa_uid` for accurate count of sync actions.</td>
    <td>Cat 2: Interaction Data</td>
    <td><a href="https://bugzilla.mozilla.org/show_bug.cgi?id=1923967">https://bugzilla.mozilla.org/show_bug.cgi?id=1923967</a></td>
  </tr>
  <tr>
    <td>platform</td>
    <td>Platform from which sync action was initiated: Desktop, Android, or iOS.</td>
    <td>Cat 1: Technical Data</td>
    <td><a href="https://bugzilla.mozilla.org/show_bug.cgi?id=1923967">https://bugzilla.mozilla.org/show_bug.cgi?id=1923967</a></td>
  </tr>
  <tr>
    <td>device_tyoe</td>
    <td>Type of device being used to make Sync action. Desktop PC, Tablet, Mobile.</td>
    <td>Cat 1: Technical Data</td>
    <td><a href="https://bugzilla.mozilla.org/show_bug.cgi?id=1923967">https://bugzilla.mozilla.org/show_bug.cgi?id=1923967</a></td>
  </tr>
  <tr>
    <td>collection</td>
    <td>Related individual collection where sync activity took place. Includes bookmarks, history, forms, prefs, tabs, and passwords.</td>
    <td></td>
    <td><a href="https://bugzilla.mozilla.org/show_bug.cgi?id=1923967">https://bugzilla.mozilla.org/show_bug.cgi?id=1923967</a></td>
  </tr>
    <tr>
    <td>submission_timestamp</td>
    <td>Glean internal submission timestamp of metric ping.</td>
    <td></td>
    <td><a href="https://bugzilla.mozilla.org/show_bug.cgi?id=1923967">https://bugzilla.mozilla.org/show_bug.cgi?id=1923967</a></td>
  </tr>
</table>

6) Please provide a link to the documentation for this data collection which describes the ultimate data set in a public, complete, and accurate way.
 * Often the Privacy Notice for your product will link to where the documentation is expected to be.
 * Common examples for Mozilla products/services:
    * If this collection is Telemetry you can state "This collection is documented in its definitions files Histograms.json, Scalars.yaml, and/or Events.yaml and in the Probe Dictionary at https://probes.telemetry.mozilla.org."
    * If this data is collected using the Glean SDK you can state “This collection is documented in the Glean Dictionary at https://dictionary.telemetry.mozilla.org/"
 * In some cases, documentation is included in the project’s repository.

*
* [Glean Metrics & Pings Definitions]()
* [Decision Brief - Server-Side Sync Usage Attribution from Mozilla Accounts](https://docs.google.com/document/d/1zD-ia3fP43o-dYpwavDgH5Hb6Xo_fgQzzoWqTiX_wR8/edit)

7) How long will this data be collected?  Choose one of the following:

* We will retain the data in line with Glean's BigTable implementation, which is 6 months.
* We will want to at least retain the trends and counts of usage for at least one year.

8) What populations will you measure?

* We will measure all active use of Sync (attributed from the server-side). This includes all countries, locales and release channels which Sync is used.

9) If this data collection is default on, what is the opt-out mechanism for users?
* Opting out would involve a user not being signed into Sync

10) Please provide a general description of how you will analyze this data.

* Measurement of Daily Active Use:
- Based on the individual unique user_id (`hashed_fxa_uid`), we will count a single active user whenn, during a 24-hour period, the user makes a request to any collection within their Sync account (including bookmarks, tabs, prefs, etc.).

- A user that initiates multiple requests to the same collection or other collections will only be counted once. This is why the unique `hashed_fxa_uid` is required to derive uniqueness. Active use can also be across several devices, so an accompanying `platform` key defines whether the user initiated the action on Desktop, iOS or Android. Actions taken across multiple platforms for the same user should only count as an active user once, within a 24-hour period.

11) Where do you intend to share the results of your analysis?

* The Sync Ecosystem Team, related product partners, and the broader PXI organization will use this data.
* We will be using Glean for our telemetry emission, aggregation and analysis. Queries will be defined through Google BigQuery and visualized using Glean's provided tools.

12) Is there a third-party tool (i.e. not Glean or Telemetry) that you are proposing to use for this data collection? If so:

* No, since we are using Glean, this is internal.
