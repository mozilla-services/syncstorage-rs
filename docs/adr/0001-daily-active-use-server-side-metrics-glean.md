# Measuring Server-Side Daily Active Use (DAU) With Glean

* Status: approved
* Deciders: Taddes Korris, David Durst, JR Conlin, Phillip Jenvey
* Date: 2024-10-09

Technical Story: 
[Jira Epic Url](https://mozilla-hub.atlassian.net/browse/SYNC-4389)


## Context and Problem Statement

There is an organizational requirement for each service to be able to measure its own DAU (Daily Active Users). Sync historically measured DAU via FxA through browser login. With the addition of Relay, device backups/migrations, etc. this will no longer an be accurate reflection of Sync usage, nor a direct measurement originating within Sync. This is due to increased browser sign-ins with the potential of overcounting, or not counting those logging into the browser without Sync enabled. See [Decision Brief - Server-Side Sync Usage Attribution from Mozilla Accounts](https://docs.google.com/document/d/1zD-ia3fP43o-dYpwavDgH5Hb6Xo_fgQzzoWqTiX_wR8/edit?tab=t.0#heading=h.mdoaoiyvqgfo).

The goal is to measure DAU (and subsequently WAU & MAU) by emitting metrics from syncstorage-rs itself. This requires the following data:
* User identifier (hashed_fxa_uid)
* Timestamp
* Platform (Desktop, Fenix, iOS, Other)
* Device Family (Desktop, Mobioe, Tablet, Other)
* Device ID (hashed_device_id) for opt-out/deletion
 
## Decision Drivers

1. Simplicity of implementation, not only for internal metric emission, but for processing and querying.
2. Reduction of complexity and load on team to process and make sense of data. Ideally there is existing infrastructure to reduce the possible significant work to calculate DAU.
3. Glean is an internal tool successfully used by many teams at Mozilla.
4. The Glean team has extensive experience and can provide considerable support in establishing application metrics.
5. Extensibility: ability to expand scope of metrics to other measurements and apply methodology to other services in Sync ecosystem.

## Considered Options

* A. Glean Parser for Rust - Contribute to glean team repo by implementing Rust server
* B. Custom Glean Implementation - Our own custom implementation, internal only to Sync
* C. StatsD and Grafana

## Decision Outcome

Chosen option:

* A. "Glean Parser for Rust: for server-side measurement of DAU"

In researching possible implementation methods, it became clear that many options did not offer us the ease and flexibility to reconcile the data after emission. This is why Glean is recommended as a clear frontrunner, due to its rich tooling in aggregating, querying, and visualizing data.

However, this left an additional decision to either 1. implement our own custom Glean implementation to emit "Glean-compliant" output, or 2. to contribute to the Glean team's `glean_parser` repository for server-side metrics. The `glean_parser` is used for all server implementations of Glean, since the SDK is only available for client-side applications. Currently, Rust is not supported in the server-side.

In consultation with the Glean team, we determined that avoiding our custom implementation and opting for the general-purpose `glean_parser` for Rust was the ideal solution. The Glean team does/did not have capacity to implement the Rust `glean_parser` feature, so we had to decide if we were willing to take on the effort. 

Agreeing to implement this feature gave us the benefits of a general purpose solution that solved our immediate challenge but also solved for future possible challenges, all while making this feature available to other Mozilla teams.

## Pros and Cons of the Options

### A. Glean Parser for Rust

Glean is a widely used tool at Mozilla and provides us with a solution to the given issue with possible extensibility in the future. There would be some challenges related to the required development effort and coordination with the Glean team. However, the potential for positive impact within our team and the organization is significant: all Rust server applications will be able to use Glean with full server support, our possible intention to integrate Glean into Push is made easier, and this is done in partnership with the Glean team.

#### Pros

* Makes Glean compatible for all Rust server applications going forward.
* Preferred option of the Glean team.
* Glean team believes, based on FxA metrics volume, that our volume will not be a problem (180-190K per minute).
* A collection of metrics, emitted as a single "Ping Event" make querying of related data simpler.
* Core of Glean's purpose is to measure user interactions and the rich metadata that accompanies it.
* Capacity for future expansion of application metrics within Sync, beyond DAU.
* Easier to query, have data team support to set up queries.
* Use of standardized Mozilla tooling.
* Establishment of team knowledge of using Glean.
* Establishment of server-side Rust best practices, leading to easier development for backend Rust applications.
* Transparency in data review process with consideration made to minimum collection of data. This is as a result of defining `metrics.yaml` files that require data steward review to implement.

#### Cons

* We have to write the Rust-compatible code. The `glean_parser` tool used by other server-side Glean applications is currently not supported for Rust.

### B. Custom Glean Implementation 

This was originally the desired approach to measuring DAU via Glean.  It was predicated on the ease of prototyping a custom implementation that imitated the Glean team's logic to create "Glean-compliant" output.  However, in consulting with the Glean team and evaluating the pros and cons of this approach, it became clear this approach had considerably more drawbacks than implementing the `glean_parser` for Rust. These drawbacks were a lack of testing, validation, less support from the Glean team, and the potential problems with maintenance and adding Glean metrics in the future.

#### Pros
* Gives team control over implementation and allows us to customize the Glean logging as we see fit.
* Effort is smaller because it doesn't involve our integrating with an existing repo we are not familiar with. Ex. the templating logic and libraries in the `glean_parser`.
* Easy to prototype and make changes.


#### Cons
* There is no built-in testing suite or validation, so this would put a larger development burden on us and require the Glean team's review.
* Lack of testing and validation means higher likelihood of bugs.
* If we decide to add new Glean metrics in the future, this may break the custom implementation and impose a greater maintenance overhead.
* Time required to understand the Glean team's implementations in order to replicate behavior and data structures.
* Likely won't have same support from Glean team as it is not related to their implementation.

### C. StatsD, Grafana, InfluxDB

StatsD and Grafana offer us core application metrics and service health. While we use this frequently, it doesn't neatly fit the measurement requirements we have for DAU and would likely be very difficult to process via queries. This is because such application metrics are geared towards increment counters, response codes, and timers. Given DAU is a user-initiated interaction, and we need to query unique events based on the `hashed_fxa_id`, this is not suitable for InfluxDB/StatsD. Figuring out how to query this data poses challenges as it has not been implemented for such a use case. 

#### Pros

* Already utilized for core service metrics (status codes, API endpoint counts, cluster health, etc).
* Well understood and used.
* Possible for SRE for more complex queries.

#### Cons

* StatsD is not a good format for measuring something like user interactions.
* Somewhat opaque and complicated query logic required.
* Significant difficulty in aggregation and reconciliation logic. 
* May not scale well given number of events.
* Likely a considerable overhead in understanding how to make sense of data.
* Heavier load for team to manage data processing, as this approach has not been tried.


## Links 

* [Decision Brief - Server-Side Sync Usage Attribution from Mozilla Accounts](https://docs.google.com/document/d/1zD-ia3fP43o-dYpwavDgH5Hb6Xo_fgQzzoWqTiX_wR8/edit?tab=t.0#heading=h.mdoaoiyvqgfo)
* [FxA DAU Metric in Redash](https://sql.telemetry.mozilla.org/queries/101007/source?p_end%20date=2024-06-26&p_start%20date=2024-05-01#248905)
* [Working Document](https://docs.google.com/document/d/1Tk4VIuQZcn8IG-UI38kziZn5e-FMOI0Z-VrvaYTI1SM/edit#heading=h.b0mqx1fng4wa)
* [Sync Ecosystem Infrastructure and Metrics: KPI Metrics](https://mozilla-hub.atlassian.net/wiki/spaces/CLOUDSERVICES/pages/969834589/Establish+KPI+metrics+DAU+Retention)
* [Integrating Glean](https://mozilla.github.io/glean/book/user/adding-glean-to-your-project/rust.html)
* [Glean Metrics](https://mozilla.github.io/glean/book/reference/metrics/index.html)