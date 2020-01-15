#!/bin/bash
# Run this script in the project root
args=$*
RUST_LOG=debug
#FEATURES="--features no_auth"
RUN_CMD=
#RUN_CMD= target/debug/syncstorage
FEATURES=
RUST_LOG=error,syncstorage=debug RUST_TEST_THREADS=1 cargo run $FEATURES -- --config spanner_sync.ini $args

