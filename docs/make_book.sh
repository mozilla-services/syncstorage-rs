#!/bin/bash
echo Generating the cargo docs
cargo doc --all-features --workspace --no-deps

echo Generating mdbook
mdbook build
mdbook-mermaid install .

echo Generate the API docs
mkdir -p output/api
cargo doc --all-features --workspace --no-deps
cp -r ../target/doc/* output/api
