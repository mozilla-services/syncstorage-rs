# Measuring Server-Side Daily Active Use (DAU) With Glean

* Status: proposed
* Deciders: Taddes Korris, David Durst, JR Conlin, Phillip Jenvey
* Date: 2024-10-09

Technical Story: 
[Jira Epic Url](https://mozilla-hub.atlassian.net/browse/SYNC-4389)


## Context and Problem Statement

There is a requirement to move away from the current measurement of Sync Daily Active Use (DAU), which measures usage via FxA.
The addition of Relay and additional authentication paths through FxA means the metric will no longer an be accurate reflection of Sync usage. Furthermore, there is a broader organizational movement towards measuring DAU within services themselves. 

The goal is to measure DAU (and subsequently WAU & MAU) by emitting metrics from syncserver-rs itself. This requires the following data:
* User identifier (hashed_fxa_uid)
* Timestamp
* Platform (from UserAgent: Desktop, Fenix, iOS)
* Collection updated
 
In researching possible implementation methods, it became clear that many options did not offer us the ease and flexibility to reconcile the data after emission. This is why Glean is recommended as a clear frontrunner. This is not without some drawbacks, but they are minimal compared to other options that would, for example, require considerable data processing and querying difficulties. There is support for this implementation from the Glean team and active support in the process.

## Decision Drivers

1. Simplicity of implementation, not only for internal metric emission, but for processing and querying.
2. Glean is an internal tool successfully used by many teams at Mozilla.
3. The Glean team has extensive experience and can provide considerable support in establishing application metrics.
4. Extensibility: ability to expand scope of metrics to other measurements and apply methodology to other services in Sync ecosystem.

## Considered Options

* A. Glean for server-side measurement of DAU
* B. StatsD and Grafana
* C. Sql/Redash

## Decision Outcome

Chosen option:

* A. "Glean for server-side measurement of DAU"

The use of Glean appears the best choice for measuring internal DAU metrics.  It meets our requirements and provides us with needed support on the data processing side. It also provides considerable support from the Glean team to implement this in a thoughtful manner.  There are some challenges with this implementation (more below), namely in this being a greenfield attempt at Rust server-side metrics, however the pros outweigh the cons. Other metrics implementations like StatsD and Grafana cannot be easily used to measure and aggregate this data. Additionally, it adds considerable overhead in determining how to query the data and reconcile ping emissions of Sync events. Having dedicated organizational support means we establish best practices.

### Positive Consequences

* Satisfaction of requirement to measure internal DAU metrics.
* Capacity for future expansion of application metrics within Sync beyond DAU.
* Prepares for implementation of same measurements in autopush, also using Glean.
* Use of standardized Mozilla tooling.
* Establishment of team knowledge of using Glean.
* Establishment of server-side Rust best practices, leading to easier development for backend Rust applications.
* Transparency in data review process with consideration made to minimum collection of data.

### Negative Consequences

* The `glean_parser` tool used by other Glean applications is currently not supported for Rust. Furthermore, client applications can use the Glean SDK and this is also not an option for us.
* Server side metrics have not yet been implemented for a Rust server application of this kind, so this is new territory.
* There is added complexity of data review process and registration of the application to Glean's probe scraper.
* Potential delays and challenges in new implementation.

## Pros and Cons of the Options

### [option A]

[example | description | pointer to more information | …] <!-- optional -->

#### Pros

* [argument for]
* [argument for]
* … <!-- numbers of pros can vary -->

#### Cons

* [argument against]
* … <!-- numbers of cons can vary -->

### [option B]

[example | description | pointer to more information | …] <!-- optional -->

#### Pros

* [argument for]
* [argument for]
* … <!-- numbers of pros can vary -->

#### Cons

* [argument against]
* … <!-- numbers of cons can vary -->

### [option C]

[example | description | pointer to more information | …] <!-- optional -->

#### Pros

* [argument for]
* [argument for]
* … <!-- numbers of pros can vary -->

#### Cons

* [argument against]
* … <!-- numbers of cons can vary -->

## Links 

* [Discussion Document](https://docs.google.com/document/d/1Tk4VIuQZcn8IG-UI38kziZn5e-FMOI0Z-VrvaYTI1SM/edit#heading=h.b0mqx1fng4wa)
* [FxA DAU Metric in Redash](https://sql.telemetry.mozilla.org/queries/101007/source?p_end%20date=2024-06-26&p_start%20date=2024-05-01#248905)
* [Sync Ecosystem Infrastructure and Metrics: KPI Metrics](https://mozilla-hub.atlassian.net/wiki/spaces/CLOUDSERVICES/pages/969834589/Establish+KPI+metrics+DAU+Retention)
