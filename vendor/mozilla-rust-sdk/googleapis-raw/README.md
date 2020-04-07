# googleapis-raw

These are raw bindings for Google APIs based on [`grpcio`](https://github.com/pingcap/grpc-rs).

## Documentation

To generate and open documentation, run:

```
cargo doc --open
```

## Examples

To run hand-written examples, try:

```
cargo run --example spanner-query
cargo run --example bigtable-query
```

## Setting up Google Cloud SDK

Before running examples, make sure the Google Cloud SDK is set up in your environment.
If you need help, follow these guides:

1. [Installing the SDK](https://cloud.google.com/sdk/install)
2. [Setting up the SDK](https://cloud.google.com/sdk/docs/initializing)
3. [Getting started with Authentication](https://cloud.google.com/docs/authentication/getting-started)

As a final check:

* Run `gcloud info` to see the SDK configuration.
* Run `echo $GOOGLE_APPLICATION_CREDENTIALS` to verify that the credentials have been set up.
* Run `gcloud auth login` to login into Google Cloud

There is Docker setup available that installs all necessary tools, libraries, see the [README](../docker/README.md)
inside the `./docker` folder.


Useful links for setting up specific Google services:

* [Setting up Spanner](https://cloud.google.com/spanner/docs/getting-started/set-up)
* [Installing the Cloud SDK for Cloud Bigtable](https://cloud.google.com/bigtable/docs/installing-cloud-sdk)
* [Quickstart using the Pub/Sub CLI tool](https://cloud.google.com/pubsub/docs/quickstart-cli)

## Generating Rust bindings from `.proto` files

**NOTE:** You may need to update these bindings after protobuf updates.
The process requires an update to the `protobuf-codegen` cargo plugin.

This requires the installation of [protobuf](https://google.github.io/proto-lens/installing-protoc.html) library
and [protoc-gen-rust](https://github.com/stepancheg/rust-protobuf/tree/master/protobuf-codegen), a plugin
for protobuf. The installed protobuf version and the protobuf crate should have the same version, e.g. `2.7.0`.
Installation of the protoc-gen-rust plugin is done via `cargo`:

```
cargo install protobuf-codegen
```
Make sure the `protoc-gen-rust` binary is available in your `$PATH` env variable.

Then:

1) In the `/src` directory, remove all the `*.rs` that are not `mod.rs` [1]
2) Run the `./generate.sh` script

ensure that a proper build works by running `cargo build`.

_[1] The `generate.sh` script will NOT generate the required `mod.rs` files for the directories. In addition, the generated rust modules will look for several modules that are `super` to their package. `protoc` may not overwrite an existing, generated rust file which could lead to complications. It's easiest if you simily leave the `mod.rs` files in place and remove the other rust files, or copy the `mod.rs` files from a backup. Running a `cargo build` will definitely identify the modules that may be missing and that you'll have to add via a line like:_

```
pub(crate) use crate::{
    empty,
    iam::v1::{iam_policy, policy},
    longrunning::operations,
};
```
_(which was taken from `src/rpc/spanner/admin/instance/v1/mod.rs`)_


## Google Cloud Console

Links to Google Cloud Console for our testing environment:

* [Spanner Console](https://console.cloud.google.com/spanner/instances?project=mozilla-rust-sdk-dev)
* [Bigtable Console](https://console.cloud.google.com/bigtable/instances?project=mozilla-rust-sdk-dev)
* [Pub/Sub Console](https://console.cloud.google.com/cloudpubsub/topic/detail/mytopic?project=mozilla-rust-sdk-dev)

## References

Google APIs and their `.proto` files:

* [Spanner](https://github.com/googleapis/googleapis/tree/master/google/spanner)
* [Bigtable](https://github.com/googleapis/googleapis/tree/master/google/bigtable)
* [Pub/Sub](https://github.com/googleapis/googleapis/tree/master/google/pubsub)

Golang clients:

* [Spanner client](https://github.com/googleapis/google-cloud-go/tree/master/spanner)
  ([docs](https://godoc.org/cloud.google.com/go/spanner))
* [Bigtable client](https://github.com/googleapis/google-cloud-go/tree/master/bigtable)
  ([docs](https://godoc.org/cloud.google.com/go/bigtable))
* [Pub/Sub client](https://github.com/googleapis/google-cloud-go/tree/master/pubsub)
  ([docs](https://godoc.org/cloud.google.com/go/pubsub))
