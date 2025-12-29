#!/bin/bash
# Generate the mdBook version of the document
echo Generating the cargo docs
cargo doc --all-features --workspace --no-deps

echo Generating mdbook
mdbook build

echo Generate the API docs
mkdir -p output/api
cargo doc --all-features --workspace --no-deps
cp -r ../target/doc/* output/api
