#!/bin/bash
echo Generating the cargo docs
cargo doc --no-deps

echo Generating mdbook
mdbook build
mdbook-mermaid install .

echo Generate the API docs
mkdir -p output/api
cargo doc --no-deps
cp -r ../target/doc/* output/api
