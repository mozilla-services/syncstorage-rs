# Measuring Server-Side Daily Active Use (DAU) With Glean

* Status: proposed
* Deciders: Taddes Korris, David Durst, JR Conlin, Phillip Jenvey
* Date: 2024-10-09

Technical Story: 
[Jira Epic Url](https://mozilla-hub.atlassian.net/browse/SYNC-4389)


## Context and Problem Statement

There is a requirement to move away from the current measurement of Sync Daily Active Use (DAU), which measures usage via FxA/Mozilla Accounts.
The addition of Relay, device backups/migrations, through FxA means the metric will no longer an be accurate reflection of Sync usage. This is due to increased browser sign-ins with the potential of overcounting, or not counting those logging into the browser without Sync enabled. Furthermore, there is a broader organizational movement towards measuring DAU within services themselves. 

The goal is to measure DAU (and subsequently WAU & MAU) by emitting metrics from syncserver-rs itself. This requires the following data:
* User identifier (hashed_fxa_uid)
* Timestamp
* Platform (from UserAgent: Desktop, Fenix, iOS)
* Collection updated
 
In researching possible implementation methods, it became clear that many options did not offer us the ease and flexibility to reconcile the data after emission. This is why Glean is recommended as a clear frontrunner. This is not without some drawbacks, but they are minimal compared to other options that would, for example, require considerable data processing and querying difficulties. There is support for this implementation from the Glean team and active support in the process.

## Decision Drivers

1. Simplicity of implementation, not only for internal metric emission, but for processing and querying.
2. Reduction of complexity and load on team to process and make sense of data. Ideally there is existing infrastructure to reduce the possible significant work to calculate DAU.
3. Glean is an internal tool successfully used by many teams at Mozilla.
4. The Glean team has extensive experience and can provide considerable support in establishing application metrics.
5. Extensibility: ability to expand scope of metrics to other measurements and apply methodology to other services in Sync ecosystem.

## Considered Options

* A. Glean
* B. StatsD and Grafana
* C. Sql/Redash

## Decision Outcome

Chosen option:

* A. "Glean for server-side measurement of DAU"

The use of Glean appears the best choice for measuring internal DAU metrics.  It meets our requirements and provides us with needed support on the data processing side. It also provides considerable support from the Glean team to implement this in a thoughtful manner.  There are some challenges with this implementation (more below), namely in this being a greenfield attempt at Rust server-side metrics, however the pros outweigh the cons. Other metrics implementations like StatsD and Grafana cannot be easily used to measure and aggregate this data. Additionally, it adds considerable overhead in determining how to query the data and reconcile ping emissions of Sync events. Having dedicated organizational support means we establish best practices.

## Pros and Cons of the Options

### A. Glean

Glean is a widely used tool at Mozilla and provides us with a solution to the given issue and possible extensibility in the future. Not without some challenges in initial implementation, but the potential for positive impact is high.

#### Pros

* Satisfaction of requirement to measure internal DAU metrics.
* Core of Glean's purpose is to measure user interactions and the rich metadata that accompanies it.
* Capacity for future expansion of application metrics within Sync beyond DAU.
* Prepares for implementation of same measurements in autopush, also using Glean.
* Easier to query.
* Use of standardized Mozilla tooling.
* Establishment of team knowledge of using Glean.
* Establishment of server-side Rust best practices, leading to easier development for backend Rust applications.
* Transparency in data review process with consideration made to minimum collection of data.

#### Cons

* The `glean_parser` tool used by other Glean applications is currently not supported for Rust. Furthermore, client applications can use the Glean SDK and this is also not an option for us.
* Server side metrics have not yet been implemented for a Rust server application of this kind, so this is new territory.
* There is added complexity of data review process and registration of the application to Glean's probe scraper.
* Potential delays and challenges in new implementation.

### StatsD and Grafana

StatsD and Grafana offer us core application metrics and service health. While we use this frequently, it doesn't neatly fit the measurement requirements we have for DAU and would likely be very difficult to process via queries.

#### Pros

* Well understood and used.
* Support for SRE for more complex queries.
* Already utilized for core metrics.

#### Cons

* StatsD is not a good format for measuring something like user interactions.
* Somewhat opaque and complicated query logic required.
* Significant difficulty in aggregation and reconciliation logic. 
* May not scale well given number of events.
* Likely considerable overhead in understanding how to make sense of data.
* Heavier load for team to manage data processing.

### SQL/Redash

The current DAU metric used from FxA uses SQL telemetry and provides the ability to query data.  It is then displayed in a redash panel. While convenient, we do not have the infrastructure in place for this option and it might involve considerable effort to establish.

#### Pros

* Used and understood within services already.
* Simple interface.

#### Cons

* Implemented strictly for accounts at present.
* Lack of clarity on how to aggregate and process data after emitted.
* Infrastructure does not exist to emit the metrics to.
* Likely considerable overhead in understanding how to make sense of data.
* Heavier load for team to manage data processing.

## Links 

* [Decision Brief - Server-Side Sync Usage Attribution from Mozilla Accounts](https://docs.google.com/document/d/1zD-ia3fP43o-dYpwavDgH5Hb6Xo_fgQzzoWqTiX_wR8/edit)
* [FxA DAU Metric in Redash](https://sql.telemetry.mozilla.org/queries/101007/source?p_end%20date=2024-06-26&p_start%20date=2024-05-01#248905)
* [Working Document](https://docs.google.com/document/d/1Tk4VIuQZcn8IG-UI38kziZn5e-FMOI0Z-VrvaYTI1SM/edit#heading=h.b0mqx1fng4wa)
* [Sync Ecosystem Infrastructure and Metrics: KPI Metrics](https://mozilla-hub.atlassian.net/wiki/spaces/CLOUDSERVICES/pages/969834589/Establish+KPI+metrics+DAU+Retention)
* [Integrating Glean](https://mozilla.github.io/glean/book/user/adding-glean-to-your-project/rust.html)
* [Glean Metrics](https://mozilla.github.io/glean/book/reference/metrics/index.html)