# Measuring Server-Side Daily Active Use (DAU) With Glean

* Status: proposed
* Deciders: Taddes Korris, David Durst, JR Conlin, Phillip Jenvey
* Date: 2024-10-09

Technical Story: 
[Jira Epic Url](https://mozilla-hub.atlassian.net/browse/SYNC-4389)


## Context and Problem Statement

There is a requiement to move away from measuring Sync Daily Active Use (DAU) via FxA.
The implementation of Relay and additional authentication paths means the metric will no
longer be accurate. Furthermore, there is a broad movement towards measuring DAU within the service itself. 

The goal is to measure DAU (and subsequently WAU & MAU) by emitting metrics from syncserver-rs itself. This involves 



## Decision Drivers <!-- optional -->

1. [primary driver, e.g., a force, facing concern, …]
2. Glean is an internal tool successfully used by many teams at Mozilla.
3. The Glean team has extensive experience and can provide considerable support in establishing application metrics.

## Considered Options

* A. Glean for server-side measurement of DAU
* B. StatsD and Grafana
* C. Sql/Redash


## Decision Outcome

Chosen option:

* A. "Glean for server-side measurement of DAU"

[justification. e.g., only option, which meets primary decision driver | which resolves a force or facing concern | … | comes out best (see below)].

### Positive Consequences <!-- optional -->

* [e.g., improvement of quality attribute satisfaction, follow-up decisions required, …]
* …

### Negative Consequences <!-- optional -->

* [e.g., compromising quality attribute, follow-up decisions required, …]
* …

## Pros and Cons of the Options <!-- optional -->

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
